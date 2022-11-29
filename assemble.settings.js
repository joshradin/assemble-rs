settings.root_project.name = "assemble";
settings.include(
    "crates/assemble-core",
    "crates/assemble-std"
);
settings.include("crates/assemble").name = "assemble-exec"