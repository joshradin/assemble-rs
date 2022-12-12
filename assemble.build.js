require("tasks/task")

logger.info("Hello, {}", "world");
logger.error("ERROR!");
print("hello, world from print")
eprint("hello, world from eprint")

class BuildTask extends DefaultTask {
    constructor(name) {
        super(name);

        this.doFirst(() => {

        })
    }

    build() {

    }
}

print("running in project: {}", project)
let build_task = project.register("sayHello", BuildTask);
build_task.configure((task) => { });
build_task.configure((task) => {
    task.doFirst(
        (task) => {
            print("{}", task)
        }
    )
    task.build()
})
logger.info("{}", build_task)