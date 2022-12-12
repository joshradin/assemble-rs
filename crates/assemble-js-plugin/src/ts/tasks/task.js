"use strict";
class DefaultTask {
    constructor(name) {
        this.name = name;
        this.my_actions = [];
        this.doFirst(this.task_action);
    }
    actions() {
        return this.my_actions;
    }
    doFirst(callback) {
        this.my_actions.unshift(callback);
    }
    doLast(callback) {
        this.my_actions.push(callback);
    }
    toString() {
        return `${this.name}`;
    }
    execute() {
        for (let action of this.actions()) {
            action(this);
        }
    }
    task_action() {
    }
}
class WriteTask extends DefaultTask {
    constructor(name, msg) {
        super(name);
        this.msg = msg;
    }
}
