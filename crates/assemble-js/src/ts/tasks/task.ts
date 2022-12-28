interface Task {

    /**
     * The main execution of the task
     */
    task_action(): void;
}

class Empty implements Task {
    task_action(): void {
    }

}