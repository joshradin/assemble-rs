require("project")

/**
 * The main project being used
 */
declare const project: Project;

function task<T extends Task>(name: string, cons: () => T): TaskHandle<T> {
    project.register(name, cons)
}