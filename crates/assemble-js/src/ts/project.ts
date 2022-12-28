require("tasks/task")
require("identifier")

interface Project {
    id(): Id;
    register<T extends Task>(name: string, cons: () => T): TaskHandle<T>;
}

interface TaskHandle<T extends Task> {
    id() : Id;
    configure<R extends T>(fun: (task: Executable<R>, project: Project) => void): void;
}