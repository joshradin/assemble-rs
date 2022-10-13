interface TaskAction<in T> {
    (arg0: T): void;
}

type Action<T> = TaskAction<T>


interface Task {
    readonly name: String

    actions(): Action<this>[];


    doFirst(callback: Action<this>): void;
    doLast(callback:  Action<this>): void;

}

function TaskAction() {
    return function (target: any, propertyKey: string, descriptor: PropertyDescriptor) {

    };
}

class DefaultTask implements Task {
    readonly name: String;
    private before: Action<this>[] = [];
    private after: Action<this>[] = [];


    constructor(name: String) {
        this.name = name;
    }

    doFirst<T >(callback: Action<this>): void {
        this.before.unshift(callback);
        callback(this)
    }

    doLast(callback: Action<this>): void {
        this.after.push(callback);
        callback(this)
    }


    actions(): Action<this>[] {
        const output: Action<this>[] = [];
        output.push(...this.before);
        output.push(...this.after);
        return output;
    }


}

class WriteTask extends DefaultTask {
    private to_write: string = "";
    
}
