require("tasks/task")
require("identifier")

declare class Project {
    id(): Id;
    register<T extends Task>(name: string, cons: () => T): TaskProvider<T>;
}

declare class TaskProvider<T extends Task> {
    id() : Id;
    configure<R extends T>(fun: (task: R) => void): void;
}