declare interface ProjectObj {
    id(): Id;
}

class Project {
    private obj: ProjectObj

    constructor(project: ProjectObj) {
        this.obj = project
    }

    id(): Id {
        return this.obj.id()
    }

}

