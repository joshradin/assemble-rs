use super::Task;
use crate::identifier::TaskId;
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::project::{ProjectError, ProjectResult, SharedProject};
use crate::task::{Action, BuildableTask, Empty, ExecutableTask, HasTaskId, TaskAction, TaskOrdering, TaskOrderingKind};
use crate::{BuildResult, Project};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use crate::exception::BuildException;

/// The wrapped task itself
pub struct Executable<T : Task> {
    pub task: T,
    project: SharedProject,
    task_id: TaskId,
    first: Mutex<Vec<Action<T>>>,
    last: Mutex<Vec<Action<T>>>,
    task_ordering: Vec<TaskOrdering>,
    queried: AtomicBool
}

assert_impl_all!(Executable<Empty> : Send);

impl<T: 'static + Task + Send + Debug> Executable<T> {
    pub fn new(shared: SharedProject, task: T, task_id: TaskId) -> Self {
        Self {
            task,
            project: shared,
            task_id,
            first: Default::default(),
            last: Default::default(),
            task_ordering: Default::default(),
            queried: AtomicBool::new(false)
        }
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
        match self.queried.compare_exchange(false, true, Ordering::Release, Ordering::Relaxed) {
            Ok(false) => {
                let first: Vec<_> = self.first.lock()?
                    .drain(..)
                    .rev()
                    .collect();
                let last: Vec<_> = self.last.lock()?
                    .drain(..)
                    .collect();
                Ok((first, last))
            }
            Ok(true) => unreachable!(),
            Err(_) => {
                Err(ProjectError::ActionsAlreadyQueried)
            }
        }
    }

    fn actions(&self) -> ProjectResult<Vec<Box<dyn TaskAction<T>>>> {
        let mut output: Vec<Box<dyn TaskAction<T>>> = vec![];
        let (first, last) = self.query_actions()?;
        output.extend(first.into_iter().map(|a| Box::new(a) as Box<dyn TaskAction<T>>));
        output.push(Box::new(T::task_action));
        output.extend(last.into_iter().map(|a| Box::new(a) as Box<dyn TaskAction<T>>));
        Ok(output)
    }
    pub fn project(&self) -> &SharedProject {
        &self.project
    }
}

impl<T: Task + Debug> Debug for Executable<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Executable")
            .field("task", &self.task)
            .field("id", &self.task_id)
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
        let mut built_by = BuiltByContainer::new();
        for ordering in self.task_ordering.iter()
            .filter(|b| b.ordering_kind() == &TaskOrderingKind::DependsOn) {
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


    fn built_by(&self, project: &Project) -> BuiltByContainer {
        todo!()
    }
}

impl<T: 'static + Task + Send + Debug> ExecutableTask for Executable<T> {


    fn execute(&mut self, project: &Project) -> BuildResult {
        for mut action in self.actions()? {
            let result: BuildResult = action.execute(self, project);
            match result {
                Ok(()) => {}
                Err(BuildException::StopAction) => {
                    continue
                }
                Err(BuildException::StopTask) => {
                    return Ok(());
                }
                Err(e) => return Err(e)
            }
        }
        Ok(())
    }


}
