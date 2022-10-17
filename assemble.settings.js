settings.root_project.name = "assemble";
settings.include(
    "crates/assemble-core",
    "crates/assemble",
    "crates/assemble-std"
);
logger.info(String(settings));