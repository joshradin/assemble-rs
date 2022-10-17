use rquickjs::FromJs;


#[derive(Debug, FromJs)]
pub struct Settings {
    pub root_project: ProjectDescriptor
}

#[derive(Debug, FromJs)]
pub struct ProjectDescriptor {
    pub name: String,
    pub path: String,
    pub children: Vec<ProjectDescriptor>
}