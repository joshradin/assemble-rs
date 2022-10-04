use std::collections::HashMap;
use std::path::PathBuf;
use assemble_core::lazy_evaluation::{Prop, Provider};
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_core::__export::{InitializeTask, ProjectResult, TaskId};
use assemble_core::file_collection::FileSet;
use assemble_core::task::TaskAction;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::task::work_handler::output::Output;
use assemble_core::task::TaskIO;
use assemble_macros::{CreateTask, TaskIO};


#[test]
fn can_reuse_basic_output() {
    #[derive(Debug, Default, TaskIO)]
    struct CountLines {
        #[input(file)]
        path: Prop<PathBuf>,
        #[output]
        lines: Prop<usize>
    }

    impl UpToDate for CountLines {}
    impl InitializeTask for CountLines {}

    impl Task for CountLines {
        fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
            todo!()
        }
    }


    let mut mapping = HashMap::new();
    mapping.insert("lines".to_string(), ron::to_string(&15).expect("couldn't serialize"));

    let ref output = Output::new(
        FileSet::new(),
        mapping
    );

    let mut count_lines = CountLines::default();
    count_lines.recover_outputs(output);

    assert_eq!(
        count_lines.lines.get(), 15, "Should be set to 15 after output recovered"
    );

}