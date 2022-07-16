use assemble_core::Project;
use assemble_core::task::Empty;

#[test]
fn lazy_evaluation() {

    let project = Project::temp(None);
    println!("project: {}", project);

    let container = project.tasks();

}