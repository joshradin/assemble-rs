use super::Task;
use crate::defaults::tasks::Empty;
use crate::exception::BuildException;
use crate::identifier::TaskId;
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::project::{ProjectError, ProjectResult, SharedProject};
use crate::task::flags::{OptionDeclaration, OptionDeclarations, OptionsDecoder};
use crate::task::previous_work::WorkHandler;
use crate::task::up_to_date::{UpToDate, UpToDateContainer};
use crate::task::{
    Action, BuildableTask, ExecutableTask, HasTaskId, TaskAction, TaskOrdering, TaskOrderingKind,
};
use crate::{BuildResult, Project};
use colored::Colorize;
use log::{debug, error, info, trace};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The wrapped task itself
pub struct Executable<T: Task> {
    pub task: T,
    project: SharedProject,
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
        let id = task_id.as_ref().clone();
        Self {
            task,
            project: shared,
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

    pub(crate) fn initialize(&mut self, project: &Project) -> ProjectResult {
        T::initialize(self, project)
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
        F: Send,
    {
        let action = Action::new(a);
        self.first.lock()?.push(action);
        Ok(())
    }

    pub fn do_last<F>(&mut self, a: F) -> ProjectResult
    where
        F: Fn(&mut Executable<T>, &Project) -> BuildResult + 'static,
        F: Send,
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
            Err(_) => Err(ProjectError::ActionsAlreadyQueried),
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
    pub fn project(&self) -> &SharedProject {
        &self.project
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
        built_by
    }
}

impl<T: 'static + Task + Send + Debug> HasTaskId for Executable<T> {
    fn task_id(&self) -> &TaskId {
        &self.task_id
    }
}

impl<T: 'static + Task + Send + Debug> BuildableTask for Executable<T> {
    fn ordering(&self) -> Vec<TaskOrdering> {
        self.task_ordering.clone()
    }
}

impl<T: 'static + Task + Send + Debug> ExecutableTask for Executable<T> {
    fn options_declarations(&self) -> Option<OptionDeclarations> {
        T::options_declarations()
    }

    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()> {
        self.task.try_set_from_decoder(decoder)
    }

    fn execute(&mut self, project: &Project) -> BuildResult {
        let up_to_date = self.handler_up_to_date();

        let input = self.work.get_input().clone();
        trace!("input: {:#?}", input);

        let inputs_up_to_date = if up_to_date || self.up_to_date.len() == 0 {
            if input.any_inputs() {
                let previous = self.work.try_get_prev_input();
                trace!("prev input: {:#?}", previous);
                !input.input_changed(previous)
            } else {
                debug!(
                    "No inputs registered for {}, assuming up-to-date status is {}",
                    self.task_id, up_to_date
                );
                up_to_date
            }
        } else {
            false
        };

        let work = if !inputs_up_to_date {
            self.work().set_up_to_date(false);
            (|| -> BuildResult {
                let actions = self.actions()?;

                for mut action in actions {
                    let result: BuildResult = action.execute(self, project);
                    match result {
                        Ok(()) => {}
                        Err(BuildException::StopAction) => continue,
                        Err(BuildException::StopTask) => {
                            return Ok(());
                        }
                        Err(e) => return Err(e),
                    }
                }

                Ok(())
            })()
        } else {
            self.work().set_up_to_date(true);
            debug!("skipping {} because it's up-to-date", self.task_id);
            Ok(())
        };

        if input.any_inputs() {
            if work.is_ok() {
                if let Err(e) = self.work.cache_input(input) {
                    error!("encountered error while caching input: {}", e);
                }
            } else {
                if let Err(e) = self.work.remove_stored_input() {
                    error!("encountered error while removing cached input: {}", e);
                }
            }
        }

        work
    }

    fn did_work(&self) -> bool {
        self.work.did_work()
    }

    fn up_to_date(&self) -> bool {
        *self.work.up_to_date()
    }

    fn group(&self) -> String {
        self.group.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}
