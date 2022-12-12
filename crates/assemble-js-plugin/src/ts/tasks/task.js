"use strict";
class DefaultTask {
    constructor(name) {
        this.name = name;
    }
    doFirst(callback) {
    }
    doLast(callback) {
    }
    get actions() {
        return [];
    }
    toString() {
        return `${this.name}`;
    }
}
class WriteTask extends DefaultTask {
    constructor(name, msg) {
        super(name);
        this.msg = msg;
    }
}
