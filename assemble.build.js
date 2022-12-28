require("tasks/task")

logger.info("Hello, {}", "world");
logger.error("ERROR!");
print("hello, world from print")
eprint("hello, world from eprint")


print("running in project: {}", project)
let build_task = project.register("sayHello", Empty);
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