use assemble_core::task::Action;
use assemble_core::Project;
use assemble_std::tasks::Empty;

fn main() {
    let mut project = Project::default();
    project
        .task::<Empty>("hello_world")
        .unwrap()
        .configure(|_, task_opts, _| {
            task_opts.first(Action::new(|_, _| {
                println!("Hello, World!");
                Ok(())
            }))
        });
}
