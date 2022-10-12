//! Gets the typescript definitions

use include_dir::Dir;

static TRANSPILED_JAVASCRIPT: Dir<'_> = include_dir::include_dir!("$OUT_DIR/js");

#[cfg(test)]
mod tests {
    use crate::javascript::TRANSPILED_JAVASCRIPT;

    #[test]
    fn files_linked() {
        assert!(
            TRANSPILED_JAVASCRIPT.entries().len() > 0,
            "no files detected"
        );
    }

    #[test]
    fn get_project_js() {
        let project_js = TRANSPILED_JAVASCRIPT
            .get_file("project.js")
            .expect("project.js file should exist");
        let string = project_js.contents_utf8().unwrap();
        println!("{}", string);
        assert!(
            string.contains("class Project"),
            "should contain project definition"
        );
    }

    #[test]
    fn get_task_js() {
        let project_js = TRANSPILED_JAVASCRIPT
            .get_file("tasks/task.js")
            .expect("tasks/task.js file should exist");
        let string = project_js.contents_utf8().unwrap();
        println!("{}", string);
        assert!(
            string.contains("class DefaultTask"),
            "should contain project definition"
        );
    }
}
