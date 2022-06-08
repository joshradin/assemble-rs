use std::path::Path;

pub struct FileCollection {

}

pub struct Component {

}

pub trait FileFilter {

    fn accept(
        &self, file: &Path) -> bool;
}

assert_obj_safe!(FileFilter);

impl<F> FileFilter for F
where F : Fn(&Path) -> bool {
    fn accept(&self, file: &Path) -> bool {
        (self)(file)
    }
}

pub struct Invert<F : FileFilter>(F);

impl<F: FileFilter> FileFilter for Invert<F> {
    fn accept(&self, file: &Path) -> bool {
        todo!()
    }
}
