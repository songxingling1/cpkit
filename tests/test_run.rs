mod support;
#[cfg(unix)]
use crate::support::run_cpkit_with_child;
use crate::support::{
    init_workspace, prepare_configuration, prepare_header, prepare_source,
    prepare_source_without_cfg, run_cpkit,
};
#[cfg(unix)]
use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::*;
#[cfg(unix)]
use std::io::{BufRead, BufReader};

#[test]
fn success() {
    let workspace = init_workspace();
    prepare_source("valid", workspace.path());
    prepare_configuration(workspace.path());
    run_cpkit(["run", "valid.c"], workspace.path())
        .success()
        .stderr(predicate::str::contains("Execution time"))
        .stderr(predicate::str::contains("Peak memory"))
        .stderr(predicate::str::contains("Exit signal"))
        .stderr(predicate::str::contains("warning").not())
        .stderr(predicate::str::contains("error").not())
        .stdout(predicate::eq("Hello, World!"));
}

#[cfg(unix)]
#[test]
fn ctrl_c() {
    let workspace = init_workspace();
    prepare_source("longtime", workspace.path());
    prepare_configuration(workspace.path());
    let mut child = run_cpkit_with_child(["run", "longtime.c"], workspace.path());
    let pid = child.id();
    let stderr = child.stderr.as_mut();
    let reader = BufReader::new(stderr.unwrap());
    for line in reader.lines() {
        if let Ok(l) = line
            && l.contains("Running")
        {
            unsafe { libc::kill(pid as i32, libc::SIGINT) };
            break;
        }
    }
    child
        .wait_with_output()
        .expect("Failed to get output")
        .assert()
        .success()
        .stderr(predicate::str::contains("Execution time"))
        .stderr(predicate::str::contains("Peak memory"))
        .stderr(predicate::str::contains("Exit signal"))
        .stderr(predicate::str::contains("warning").not())
        .stderr(predicate::str::contains("error").not());
}

#[test]
fn interactive() {
    let workspace = init_workspace();
    prepare_source("interactive", workspace.path());
    prepare_source_without_cfg("grader", workspace.path());
    prepare_header("interactive", workspace.path());
    prepare_configuration(workspace.path());
    run_cpkit(["run", "interactive.c"], workspace.path())
        .success()
        .stderr(predicate::str::contains("Execution time"))
        .stderr(predicate::str::contains("Peak memory"))
        .stderr(predicate::str::contains("Exit signal"))
        .stderr(predicate::str::contains("warning").not())
        .stderr(predicate::str::contains("error").not())
        .stdout(predicate::eq("Interactive!"));
}
