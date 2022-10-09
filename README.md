# `assemble.rs`

![GitHub Workflow Status](https://img.shields.io/github/workflow/status/joshradin/assemble-rs/Rust)
![Crates.io](https://img.shields.io/crates/v/assemble-core)
![Crates.io](https://img.shields.io/crates/d/assemble-core)
![GitHub issues](https://img.shields.io/github/issues/joshradin/assemble-rs)


A new building tool, built in rust. Inspired by `make` and `gradle`.

![welcome task](resources/help_task.gif)


## Basic Concept

Creates a binary that can build a project by running tasks. This
binary is created by this application and can be **easily** configured
by end users. 

There should be a multitude of ways of creating tasks for these projects
that doesn't require extensive knowledge of rust.

The api for `assemble.rs` should provide ways of defining tasks,
adding tasks to the build reactor, and checking on the result of the tasks.

Creating a build binary should be as easy as running

```shell
assemble-daemon startup_api # create the initial binary
```
Followed by
```shell
./assemble-daemon build
```
or
```shell
assemble-daemon build
```

### Justification
Gradle is cool, but has certain limitations that could be avoided.
- Requires a java installation
- Requires internet connection for first build in order to get the wrapper
- Many concepts are java specific, such as sourceSets

The aim for this project would be to address these issues while also providing
other benefits for users.

## Tasks

All tasks should have the following capabilities:

- Run some actions
- Define a set of inputs and outputs that can interacted with by other
tasks.
- Set task order:
  - Strict depends on - _depended on task always runs_
  - Strict finalized by - _finalizer task always run after task_
  - Run after - _task should run after a task, but doesn't force the task to run_
  - Run before - _task should run before a task, but doesn't force the task to run_
- Report result of the Task

Once the project has been configured, the only parts of the task that should be
mutable is the task properties.

## Components

Besides tasks, here are some ideas for potential critical API objects

### `Project`

`Project`s should hold the current state of the project as whole. This will
include the actual project layout, tasks, and extensions to the project.

### `BuildException`

This should represent that somehow the execution of building the project went wrong. This should be an 
Enum type to support multiple ways of representing states. The main exception types should be:
 - Stop Action - _stop the current action of the task and move on to the next_
 - Stop Task - _stops the task without causing a failure_
 - Task Failed - _The task has failed, and should fail the build_


### `TaskAction`

Task actions should, in essence, be functions that take the form of 
```rust
fn(&mut Self : ExecutableTask<Task>, &Project) -> Result<(), AssembleException>;
```

## Freight and Parallelism

Tasks can be run in parallel with each-other, and freight automatically determines
an order to execute tasks such that any tasks that aren't dependent on each-other
can execute at the same time. 

![tasks in parallel](resources/parallelism.gif)

## Bin Maker

Not implemented yet