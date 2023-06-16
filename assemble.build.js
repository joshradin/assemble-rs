require("tasks/task")



project.register("hello", Empty).configure((task) => {
    logger.info("task: {}", task.task().toString())
    logger.info("project: {}", project)
    task.doFirst(task => {
        // logger.info("hello, world!")
    })
})

class Tasks {
    create() {

    }
}
