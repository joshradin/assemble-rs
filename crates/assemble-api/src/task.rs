use crate::exception::BuildException;
use crate::project::Project;
use crate::utilities::AsAny;
use std::any::Any;
use std::cell::{Ref, RefMut};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::{Index, IndexMut};

pub trait TaskAction {
    fn execute(&self, task: &dyn Task, project: &Project) -> Result<(), BuildException>;
}

assert_obj_safe!(TaskAction);

impl<F> TaskAction for F
where
    F: Fn(&dyn Task, &Project) -> Result<(), BuildException>,
{
    fn execute(&self, task: &dyn Task, project: &Project) -> Result<(), BuildException> {
        (&self)(task, project)
    }
}

pub trait Task {
    fn task_id(&self) -> &TaskIdentifier;

    fn actions(&self) -> Vec<&dyn TaskAction>;

    fn properties(&self) -> RefMut<TaskProperties>;

    fn task_dependencies(&self) -> Vec<&TaskOrdering>;
}

pub trait TaskMut: Task {
    fn first<A: TaskAction + 'static>(&mut self, action: A);
    fn last<A: TaskAction + 'static>(&mut self, action: A);

    fn depends_on<I: Into<TaskIdentifier>>(&mut self, identifier: I);
}

pub trait IntoTask: TryInto<Self::Task> {
    type Task: Task;

    fn into_task(self) -> Result<Self::Task, Self::Error> {
        self.try_into()
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
pub struct TaskIdentifier(String);

#[derive(Default)]
pub struct TaskProperties {
    inner_map: HashMap<String, Box<dyn Any>>,
}

impl TaskProperties {
    pub fn get<T: 'static>(&self, index: &str) -> Option<&T> {
        self.inner_map
            .get(index)
            .and_then(|box_ref| box_ref.downcast_ref())
    }

    pub fn get_mut<T: 'static>(&mut self, index: &str) -> Option<&mut T> {
        self.inner_map
            .get_mut(index)
            .and_then(|box_ref| box_ref.downcast_mut::<T>())
    }

    pub fn set<T: 'static>(&mut self, index: &str, value: T) {
        match self.inner_map.entry(index.to_string()) {
            Entry::Occupied(mut occ) => {
                occ.insert(Box::new(value));
            }
            Entry::Vacant(mut vac) => {
                vac.insert(Box::new(value));
            }
        };
    }
}

impl Index<&str> for TaskProperties {
    type Output = dyn Any;

    fn index(&self, index: &str) -> &Self::Output {
        &self.inner_map[index]
    }
}

#[derive(Debug)]
pub enum TaskOrdering {
    DependsOn(TaskIdentifier),
    FinalizedBy(TaskIdentifier),
    RunsAfter(TaskIdentifier),
    RunsBefore(TaskIdentifier),
}
