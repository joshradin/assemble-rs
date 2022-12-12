require("tasks/task")

logger.info("Hello, {}", "world");

class BuildTask extends DefaultTask {
    constructor(name) {
        super(name);

        this.doFirst(() => {

        })
    }
}

let build_task = new BuildTask("task")
build_task.doFirst(() => {

})

logger.info("{}", String(build_task))