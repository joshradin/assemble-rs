require("assemble")

function get_name(path: string, sep = path_separator()): string {
    const index = path.lastIndexOf(sep);
    if (index == -1) return path;
    return path.substring(index + 1);
}

class Settings {
    public root_project: ProjectDescriptor;

    constructor(root_project: string) {
        this.root_project = new ProjectDescriptor(get_name(root_project), root_project);
    }

    include(...path: [string] & string[]): ProjectDescriptor | ProjectDescriptor[] {
        if (path.length == 1) {
            return this.root_project.include(path[0]);
        } else {
            let ret: ProjectDescriptor[] = [];
            for (let i = 0; i < path.length; i++) {
                ret.push(this.root_project.include(path[i]));
            }
            return ret;
        }

    }
}

class ProjectDescriptor {
    public name: string;
    public path: string;
    children: ProjectDescriptor[];

    constructor(name: string, path: string) {
        this.name = name;
        this.path = path;
        this.children = [];
    }

    include(path: string): ProjectDescriptor {
        const ret = new ProjectDescriptor(get_name(path, "/"), this.path + "/" + path);
        this.children.push(ret);
        return ret;
    }
}

const settings = new Settings(get_name(assemble.project_dir));
