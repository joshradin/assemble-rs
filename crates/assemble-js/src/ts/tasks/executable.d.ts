interface Executable<T extends Task> {

    /**
     * Gets the identifier of the executable
     */
    id(): Id;
    doFirst(callback: (task: T) => void): void;
    doLast(callback: (task: T) => void): void

    dependsOn(deps: any): void;
}
