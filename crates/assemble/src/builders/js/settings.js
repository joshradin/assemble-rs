class ProjectDescriptor {
    constructor() {
        this.name = "";
        this.dir_path = "";
        this.children = [];
    }

    toString() {
        return "ProjectDescriptor"
    }

    project(path, callback = undefined) {
        const child = new ProjectDescriptor();
        child.name = "path"
        child.dir_path = undefined
        if (callback) {
            callback(child)
        }
        this.children += [new ProjectDescriptor()]
    }
}

class Settings {
    constructor() {
        this.root_project = new ProjectDescriptor();
    }

    toString() {
        return "Settings"
    }

    include(path, callback = undefined) {
        this.root_project.project(path, callback);
    }
}
