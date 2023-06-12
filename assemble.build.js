require("tasks/task")

// plugins({
//     'rust': 'latest'
// })
//

project.register("hello", DefaultTask).configure(task => {
    task.doFirst(task => {
        logger.info("hello, world!")
    })
})