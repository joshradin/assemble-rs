require("tasks/task")

logger.info("Hello, {}", "world");
logger.error("ERROR!");
print("hello, world from print")
eprint("hello, world from eprint")

class BuildTask extends DefaultTask {
    constructor(name) {
        super(name);
        this.doFirst(() => {
            print("do first from cons")
        })
    }

    task_action() {
        print("Wowee im gonna be in a movie")
    }
}

print("running in project: {}", project)
let build_task = project.register("sayHello", BuildTask);
build_task.configure((task) => {
    task.doLast((task) => {
        print("do last!")
    })
});
build_task.configure((task) => {
    task.doFirst(
        (task) => {
            print("{}", task);
        }
    )
})
logger.info("{}", build_task);