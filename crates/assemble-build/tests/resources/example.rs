
pub fn test1() { }

mod inner{
    pub fn test2() { }
}

#[path ="alt_path.rs"]
mod outer;