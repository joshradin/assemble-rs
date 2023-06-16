interface Executable<T extends Task> {
    group: string | null;
    description: string | null;

    /**
     * Gets the task object
     */
    task(): T;

    /**
     * Gets the identifier of the executable
     */
    id(): Id;

    doFirst(callback: (task: T) => void): void;
    doLast(callback: (task: T) => void): void

    dependsOn(deps: any): void;
}