interface Task {
    readonly name: String

    actions(): ((task: this) => void)[];

    doFirst(callback: (task: this) => void): void;
    doLast(callback: (task: this) => void): void;

    /**
     * The main execution of the task
     */
    task_action(): void;
}

class DefaultTask implements Task {

    readonly name: String;
    private my_actions: ((task: this) => void)[];


    constructor(name: String) {
        this.name = name;
        this.my_actions = [];
        this.doFirst(this.task_action)
    }

    actions(): ((task: this) => void)[] {
        return this.my_actions;
    }

    doFirst(callback: (task: this) => void): void {
        this.my_actions.unshift(callback);

    }

    doLast<T extends this>(callback: (task: this) => void): void {
        this.my_actions.push(callback);
    }

    toString(): string {
        return `${this.name}`
    }

    execute() {
        for (let action of this.actions()) {
            action(this)
        }
    }

    task_action() {

    }
}

class WriteTask extends DefaultTask {
    msg: String;

    constructor(name: String, msg: String) {
        super(name);
        this.msg = msg;
    }
}