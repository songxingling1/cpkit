mod support;
use std::{fs, os::unix::fs::PermissionsExt};

use crate::support::{
    init_workspace, prepare_configuration, prepare_header, prepare_source,
    prepare_source_without_cfg, run_artifact, run_cpkit,
};
use predicates::prelude::*;

#[test]
fn success() {
    let workspace = init_workspace();
    prepare_source("valid", workspace.path());
    prepare_configuration(workspace.path());
    run_cpkit(["compile", "valid.c", "-o", "valid"], workspace.path())
        .success()
        .stderr(predicate::str::contains("Successfully compiled"))
        .stderr(predicate::str::contains("warning").not())
        .stderr(predicate::str::contains("error").not());
    assert!(workspace.path().join("valid").is_file());
    run_artifact("valid", workspace.path())
        .success()
        .stdout(predicate::eq("Hello, World!"));
}

#[test]
fn success_without_set_output() {
    let workspace = init_workspace();
    prepare_source("valid", workspace.path());
    prepare_configuration(workspace.path());
    run_cpkit(["compile", "valid.c"], workspace.path())
        .success()
        .stderr(predicate::str::contains("Successfully compiled"))
        .stderr(predicate::str::contains("warning").not())
        .stderr(predicate::str::contains("error").not());
    assert!(workspace.path().join("a.out").is_file());
    run_artifact("a.out", workspace.path())
        .success()
        .stdout(predicate::eq("Hello, World!"));
}

#[test]
fn source_not_found() {
    let workspace = init_workspace();
    prepare_configuration(workspace.path());
    run_cpkit(["compile", "valid.c", "-o", "valid"], workspace.path())
        .failure()
        .stderr(predicate::str::contains(
            "The source file should exist and be a file.",
        ));
    assert!(!workspace.path().join("valid").exists());
}

#[test]
fn invalid_source() {
    let workspace = init_workspace();
    prepare_source("invalid", workspace.path());
    prepare_configuration(workspace.path());
    run_cpkit(["compile", "invalid.c", "-o", "invalid"], workspace.path())
        .failure()
        .stderr(predicate::str::contains("Compilation failed"))
        .stderr(predicate::str::contains(
            "error: expected ‘;’ before ‘return’",
        ));
    assert!(!workspace.path().join("valid").exists());
}

#[test]
fn cannot_get_configuration() {
    let workspace = init_workspace();
    prepare_source("valid", workspace.path());
    prepare_configuration(workspace.path());
    let mut perms = fs::metadata(workspace.path().join("cpkit.toml"))
        .expect("Failed to get config metadata")
        .permissions();
    perms.set_mode(0o000);
    fs::set_permissions(workspace.path().join("cpkit.toml"), perms)
        .expect("Failed to set permissions");
    run_cpkit(["compile", "valid.c", "-o", "valid"], workspace.path())
        .success()
        .stderr(predicate::str::contains("Successfully compiled"))
        .stderr(predicate::str::contains(
            "Failed to load configuration. Use default configuration.",
        ))
        .stderr(predicate::str::contains("error").not());
    assert!(workspace.path().join("valid").is_file());
    run_artifact("valid", workspace.path())
        .success()
        .stdout(predicate::eq("Hello, World!"));
}
#[test]
fn interactive() {
    let workspace = init_workspace();
    prepare_source("interactive", workspace.path());
    prepare_source_without_cfg("grader", workspace.path());
    prepare_header("interactive", workspace.path());
    prepare_configuration(workspace.path());
    run_cpkit(
        ["compile", "interactive.c", "-o", "interactive"],
        workspace.path(),
    )
    .success()
    .stderr(predicate::str::contains("Successfully compiled"))
    .stderr(predicate::str::contains("warning").not())
    .stderr(predicate::str::contains("error").not());
    assert!(workspace.path().join("interactive").is_file());
    run_artifact("interactive", workspace.path())
        .success()
        .stdout(predicate::eq("Interactive!"));
}
