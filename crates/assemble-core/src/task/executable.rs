use super::Task;
use crate::defaults::tasks::Empty;
use crate::exception::BuildException;
use crate::identifier::TaskId;
use crate::project::buildable::{BuiltByContainer, IntoBuildable};
use crate::project::error::{ProjectError, ProjectResult};
use crate::project::{SharedProject, WeakSharedProject};
use crate::task::action::{Action, TaskAction};
use crate::task::flags::{OptionDeclarations, OptionsDecoder};
use crate::task::task_io::TaskIO;
use crate::task::up_to_date::{UpToDate, UpToDateContainer};

use crate::task::work_handler::WorkHandler;
use crate::task::{BuildableTask, ExecutableTask, HasTaskId, TaskOrdering, TaskOrderingKind};
use crate::{BuildResult, Project};

use log::{debug, error, trace};

use std::fmt::{Debug, Formatter};

use std::ops::{Deref, DerefMut};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The wrapped task itself
pub struct Executable<T: Task> {
    pub task: T,
    project: WeakSharedProject,
    task_id: TaskId,
    first: Mutex<Vec<Action<T>>>,
    last: Mutex<Vec<Action<T>>>,
    task_ordering: Vec<TaskOrdering>,
    queried: AtomicBool,
    up_to_date: UpToDateContainer<T>,
    work: WorkHandler,

    description: String,
    group: String,
}

assert_impl_all!(Executable<Empty> : Send);

impl<T: 'static + Task + Send + Debug> Executable<T> {
    pub fn new<Id: AsRef<TaskId>>(shared: SharedProject, task: T, task_id: Id) -> Self {
        let cache_location = shared
            .with(|p| p.root_dir())
            .join(".assemble")
            .join("task-cache");
        debug!(
            "Using {:?} as cache location for {}",
            cache_location, shared
        );
        let id = task_id.as_ref().clone();

        Self {
            task,
            project: shared.weak(),
            task_id: id.clone(),
            first: Default::default(),
            last: Default::default(),
            task_ordering: Default::default(),
            queried: AtomicBool::new(false),
            up_to_date: UpToDateContainer::default(),
            work: WorkHandler::new(&id, cache_location),
            description: T::description(),
            group: "".to_string(),
        }
    }

    /// Initialize the executable.
    pub fn initialize(&mut self, project: &Project) -> ProjectResult {
        T::initialize(self, project)
    }

    /// Configures the io of this executable. Ran after the task is initialized.
    pub fn configure_io(&mut self) -> ProjectResult {
        T::configure_io(self)
    }

    pub fn depends_on<B: IntoBuildable>(&mut self, buildable: B)
    where
        B::Buildable: 'static,
    {
        trace!("adding depends ordering for {:?}", self);
        let buildable = TaskOrdering::depends_on(buildable);
        self.task_ordering.push(buildable);
    }

    pub fn do_first<F>(&mut self, a: F) -> ProjectResult
    where
        F: Fn(&mut Executable<T>, &Project) -> BuildResult + 'static,
        F: Send + Sync,
    {
        let action = Action::new(a);
        self.first.lock()?.push(action);
        Ok(())
    }

    pub fn do_last<F>(&mut self, a: F) -> ProjectResult
    where
        F: Fn(&mut Executable<T>, &Project) -> BuildResult + 'static,
        F: Send + Sync,
    {
        let action = Action::new(a);
        self.last.lock()?.push(action);
        Ok(())
    }

    fn query_actions(&self) -> ProjectResult<(Vec<Action<T>>, Vec<Action<T>>)> {
        match self
            .queried
            .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
        {
            Ok(false) => {
                let first: Vec<_> = self.first.lock()?.drain(..).rev().collect();
                let last: Vec<_> = self.last.lock()?.drain(..).collect();
                Ok((first, last))
            }
            Ok(true) => unreachable!(),
            Err(_) => Err(ProjectError::ActionsAlreadyQueried.into()),
        }
    }

    fn actions(&self) -> ProjectResult<Vec<Box<dyn TaskAction<T>>>> {
        let mut output: Vec<Box<dyn TaskAction<T>>> = vec![];
        let (first, last) = self.query_actions()?;
        output.extend(
            first
                .into_iter()
                .map(|a| Box::new(a) as Box<dyn TaskAction<T>>),
        );
        output.push(Box::new(T::task_action));
        output.extend(
            last.into_iter()
                .map(|a| Box::new(a) as Box<dyn TaskAction<T>>),
        );
        Ok(output)
    }
    pub fn project(&self) -> SharedProject {
        SharedProject::try_from(self.project.clone()).unwrap()
    }

    pub fn work(&mut self) -> &mut WorkHandler {
        &mut self.work
    }

    fn handler_up_to_date(&self) -> bool {
        if !self.task.up_to_date() {
            return false;
        }
        let handler = self.up_to_date.handler(self);
        handler.up_to_date()
    }

    pub fn set_description(&mut self, description: &str) {
        self.description = description.to_string();
    }
    pub fn set_group(&mut self, group: &str) {
        self.group = group.to_string();
    }

    /// Check to see if this task is already up-to-date before execution begins. Up-to-date handlers
    /// are ran first. If all up-to-date handlers return true, then shortcuts to returning true. If none declared, this task is always
    /// not up-to-date.
    fn up_to_date_before_execution(&self) -> ProjectResult<bool> {
        if self.up_to_date.len() > 0 && self.handler_up_to_date() {
            return Ok(true);
        }
        if !UpToDate::up_to_date(&self.task) {
            return Ok(false);
        }
        match self.work.prev_work() {
            None => Ok(false),
            Some((prev_i, prev_o)) => {
                // first run custom up-to-date checks
                if !self.handler_up_to_date() {
                    return Ok(false);
                }

                // Check if input has changed
                let current_i = self.work.get_input()?;
                if current_i.input_changed(Some(prev_i)) {
                    debug!("{} not up-to-date because input has changed", self.task_id);
                    return Ok(false);
                }

                // Check if output is not up to date
                Ok(if prev_o.up_to_date() {
                    true
                } else {
                    debug!("{} not up-to-date because output has changed", self.task_id);
                    false
                })
            }
        }
    }

    /// Add an up-to-date check
    pub fn up_to_date<F: Fn(&Executable<T>) -> bool + Send + Sync + 'static>(
        &mut self,
        configure: F,
    ) {
        self.up_to_date.up_to_date_if(configure)
    }
}

impl<T: Task + Debug> Debug for Executable<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Executable")
            .field("id", &self.task_id)
            .field("task", &self.task)
            .field("ordering", &self.task_ordering)
            .finish_non_exhaustive()
    }
}

impl<T: Task> Deref for Executable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.task
    }
}

impl<T: Task> DerefMut for Executable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.task
    }
}

impl<T: Task + Send + Debug> IntoBuildable for &Executable<T> {
    type Buildable = BuiltByContainer;

    fn into_buildable(self) -> Self::Buildable {
        trace!("Creating BuiltByContainer for {:?}", self);
        let mut built_by = BuiltByContainer::new();
        built_by.add(self.task_id.clone());
        for ordering in self
            .task_ordering
            .iter()
            .filter(|b| b.ordering_kind() == &TaskOrderingKind::DependsOn)
        {
            built_by.add(ordering.buildable().clone());
        }
        built_by.add(self.work.into_buildable());
        built_by
    }
}

impl<T: 'static + Task + Send + Debug> HasTaskId for Executable<T> {
    fn task_id(&self) -> TaskId {
        self.task_id.clone()
    }
}

impl<T: 'static + Task + Send + Debug> BuildableTask for Executable<T> {
    fn ordering(&self) -> Vec<TaskOrdering> {
        let mut explicit = self.task_ordering.clone();
        let inputs = self.work.into_buildable();
        let inputs_ordering = TaskOrdering::depends_on(inputs);
        explicit.push(inputs_ordering);
        explicit
    }
}

/// If set to true, all tasks will always run
pub static FORCE_RERUN: AtomicBool = AtomicBool::new(false);
pub fn force_rerun(value: bool) {
    FORCE_RERUN.store(value, Ordering::Relaxed)
}

impl<T: 'static + Task + Send + Sync + Debug> ExecutableTask for Executable<T> {
    fn options_declarations(&self) -> Option<OptionDeclarations> {
        T::options_declarations()
    }

    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()> {
        self.task.try_set_from_decoder(decoder)
    }

    fn execute(&mut self, project: &Project) -> BuildResult {
        let up_to_date = if FORCE_RERUN.load(Ordering::Relaxed) {
            false
        } else {
            self.up_to_date_before_execution()?
        };

        let work = if !up_to_date {
            self.work().set_up_to_date(false);
            (|| -> BuildResult {
                let actions = self.actions()?;

                for action in actions {
                    let result: BuildResult = action.execute(self, project);
                    match result {
                        Ok(()) => {}
                        Err(e) => match e.kind() {
                            BuildException::StopAction => continue,
                            BuildException::StopTask => return Ok(()),
                            BuildException::Error(_) => return Err(e),
                        },
                    }
                }

                Ok(())
            })()
        } else {
            self.work().set_up_to_date(true);
            self.work().set_did_work(false);
            debug!("skipping {} because it's up-to-date", self.task_id);

            if let Some(output) = self.work.try_get_prev_output().cloned() {
                debug!("Attempting to recover outputs from previous run");
                self.task.recover_outputs(&output)?;
                debug!("After task recovered: {:#x?}", self.task);
            }

            Ok(())
        };

        if self.work.get_input()?.any_inputs() {
            if work.is_ok() {
                if let Err(e) = self.work.store_execution_history() {
                    error!("encountered error while caching input: {}", e);
                }
            } else if let Err(e) = self.work.remove_execution_history() {
                error!("encountered error while removing cached input: {}", e);
            }
        }

        work
    }

    fn did_work(&self) -> bool {
        self.work.did_work()
    }

    fn task_up_to_date(&self) -> bool {
        *self.work.up_to_date()
    }

    fn group(&self) -> String {
        self.group.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}
