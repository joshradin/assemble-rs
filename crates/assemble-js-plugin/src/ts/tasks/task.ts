interface Task {
    readonly name: String

    get actions(): TaskAction<this>[];

    doFirst(callback: TaskAction<this>): void;
    doLast(callback: TaskAction<this>): void;

}

class DefaultTask implements Task {

    readonly name: String;


    constructor(name: String) {
        this.name = name;
    }

    doFirst(callback: TaskAction<DefaultTask>): void {
    }

    doLast(callback: TaskAction<DefaultTask>): void {
    }

    get actions(): TaskAction<DefaultTask>[] {
        return [];
    }
}

class WriteTask extends DefaultTask {
    msg: String;

    constructor(name: String, msg: String) {
        super(name);
        this.msg = msg;
    }
}
