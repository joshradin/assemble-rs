"use strict";
class DefaultTask {
    constructor(name) {
        this.before = [];
        this.after = [];
        this.name = name;
    }
    doFirst(callback) {
        this.before.unshift(callback);
    }
    doLast(callback) {
        this.after.push(callback);
    }
    get actions() {
        const output = [];
        output.push(...this.before);
        output.push(...this.after);
        return output;
    }
}
class WriteTask extends DefaultTask {
}
let def = new DefaultTask("task");
def.doFirst((task) => {
    console.log(`actions: ${task.actions}`);
});
