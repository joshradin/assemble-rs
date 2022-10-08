use assemble_build::function_finder::FunctionFinder;
use std::path::Path;

#[test]
fn find_all_functions_in_arbitrary_project_structure() {
    let test_file = Path::new("tests/resources/example.rs");

    let function_finder = FunctionFinder::find_all(test_file, "example".to_string());

    let functions = function_finder.pub_function_ids();
    let function_ids = functions.collect::<Vec<_>>();

    println!("{:#?}", function_ids);

    assert_eq!(
        function_ids,
        &[
            "example::test1",
            "example::inner::test2",
            "example::outer::test3",
            "example::outer::inner::test4",
            "example::outer::inner::other::test5",
        ]
    )
}
