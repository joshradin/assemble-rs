use assemble_core::__export::TaskId;
use assemble_core::defaults::tasks::Empty;
use assemble_core::task::{ExecutableTask, HasTaskId, ResolveExecutable, ResolveInnerTask};
use assemble_core::{Executable, Project};

#[test]
fn lazy_evaluation() {
    let project = Project::temp(None);
    println!("project: {}", project);

    let mut clean = project.tasks().register_task::<Empty>("clean").unwrap();
    clean
        .configure_with(|clean, _| {
            println!("Running first configuration action");
            clean.do_first(|task, _| {
                println!("FIRST: executing task: {}", task.task_id());
                Ok(())
            })
        })
        .unwrap();
    println!("added first configuration action");
    clean.resolve_task(&project).unwrap();
    clean
        .configure_with(|clean, _| {
            println!("Running second configuration action");
            clean.do_last(|task, _| {
                println!("LAST: executing task: {}", task.task_id());
                Ok(())
            })
        })
        .unwrap();

    let mut executable = clean.get_executable(&project).unwrap();
    project.with(|project| executable.execute(project)).unwrap();
}
