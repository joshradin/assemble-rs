interface Task<T extends Task<T>> {
    readonly name: String

    get actions(): [TaskAction<T>] | [];

    doFirst(callback: TaskAction<T>): void;
    doLast(callback: TaskAction<T>): void;

}

class DefaultTask implements Task<DefaultTask> {

    readonly name: String;


    constructor(name: String) {
        this.name = name;
    }

    doFirst(callback: TaskAction<DefaultTask>): void {
    }

    doLast(callback: TaskAction<DefaultTask>): void {
    }

    get actions(): [TaskAction<DefaultTask>] | [] {
        return [];
    }
}

class WriteTask extends DefaultTask {

}

let def = new DefaultTask("task");
def.doFirst((task: WriteTask) => {

})