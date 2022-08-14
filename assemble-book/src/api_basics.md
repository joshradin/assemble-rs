# API Basics

There are several parts that make up the basic api of assemble. The main parts
to familiarize yourself with are [`Project`][project]


[project]: #Projects


## Projects

All assemble projects are arranged in a hierarchy of projects. All projects
can have their own child projects. A basic example could be:
```
- root
  - child1
    - child2
  - child3
  - child4
    - child5
```

Actual position within the file system and corresponding point in the project
hierarchy isn't enforced, and a project can be put anywhere and made as child
project of another project. All project ids are generated like a path seperated by `:` from the
root, so `child5`'s full identifier would be `:root:child4:child5`.

