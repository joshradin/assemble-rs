class ProjectDescriptor {
    constructor() {
        this.name = "";
    }

    toString() {
        return "ProjectDescriptor"
    }
}

class Settings {
    constructor() {
        this.root_project = new ProjectDescriptor();
    }

    toString() {
        return "Settings"
    }

    include(path) {

    }
}




