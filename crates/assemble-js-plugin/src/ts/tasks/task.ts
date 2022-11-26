interface Task {
    readonly name: String

    get actions(): Action<this>[];

    doFirst(callback: Action<this>): void;
    doLast(callback: Action<this>): void;

}

class DefaultTask implements Task {

    readonly name: String;


    constructor(name: String) {
        this.name = name;
    }

    doFirst(callback: Action<DefaultTask>): void {
    }

    doLast(callback: Action<DefaultTask>): void {
    }

    get actions(): Action<DefaultTask>[] {
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