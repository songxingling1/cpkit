use crate::support::{
    init_workspace, prepare_configuration, prepare_source_without_cfg, run_cpkit,
};

mod support;
#[test]
fn success() {
    let workspace = init_workspace();
    prepare_source_without_cfg("valid", workspace.path());
    prepare_configuration(workspace.path());
    run_cpkit(["init", "valid.c"], workspace.path()).success();
    let content = std::fs::read_to_string(
        workspace
            .path()
            .join(".cpkit")
            .join("valid.c")
            .join("config.toml"),
    )
    .expect("A valid source should succeed");
    assert!(content.contains("memory-limit"));
    assert!(content.contains("time-limit"));
    assert!(content.contains("builtin"));
    assert!(content.contains("tranditional"));
}
#[test]
fn no_workspace() {
    let workspace = init_workspace();
    prepare_source_without_cfg("valid", workspace.path());
    run_cpkit(["init", "valid.c"], workspace.path())
        .failure()
        .stderr(predicates::str::contains("Workspace not found."));
}

#[test]
fn no_valid_source() {
    let workspace = init_workspace();
    prepare_configuration(workspace.path());
    run_cpkit(["init", "valid.c"], workspace.path())
        .failure()
        .stderr(predicates::str::contains("The source file is not found."));
}

#[test]
fn source_is_out_of_workspace() {
    let workspace = init_workspace();
    let dir =
        tempfile::TempDir::new_in(workspace.path()).expect("Failed to create temporary directory.");
    prepare_source_without_cfg("valid", workspace.path());
    prepare_configuration(dir.path());
    run_cpkit(["init", "../valid.c"], dir.path())
        .failure()
        .stderr(predicates::str::contains("Invalid input:"))
        .stderr(predicates::str::contains(
            "The file may no longer be in the current workspace.",
        ));
}

#[test]
fn config_is_exist() {
    let workspace = init_workspace();
    prepare_source_without_cfg("valid", workspace.path());
    prepare_configuration(workspace.path());
    let dir = workspace.path().join(".cpkit").join("valid.c");
    std::fs::create_dir_all(&dir).expect("Failed to create temporary directory");
    let file =
        std::fs::File::create(dir.join("config.toml")).expect("Failed to create temporary file.");
    file.sync_all().expect("Failed to create temporary file");
    drop(file);
    run_cpkit(["init", "valid.c"], workspace.path())
        .failure()
        .stderr(predicates::str::contains(
            "Failed to write problem configuration",
        ));
}
#[test]
fn special_problem() {
    let workspace = init_workspace();
    prepare_source_without_cfg("valid", workspace.path());
    prepare_configuration(workspace.path());
    run_cpkit(["init", "valid.c", "-i", "-c"], workspace.path()).success();
    let content = std::fs::read_to_string(
        workspace
            .path()
            .join(".cpkit")
            .join("valid.c")
            .join("config.toml"),
    )
    .expect("A valid source should succeed");
    assert!(content.contains("memory-limit"));
    assert!(content.contains("time-limit"));
    assert!(content.contains("special"));
    assert!(content.contains("interactive"));
}
