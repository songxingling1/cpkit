use assert_cmd::{Command, assert::Assert};
use std::{ffi::OsStr, path::Path};

pub fn init_workspace() -> tempfile::TempDir {
    tempfile::TempDir::new().expect("Failed to init temporary directory")
}

pub fn prepare_source(source: &str, dst: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets")
        .join(format!("{}.c", source));
    std::fs::copy(src, dst.join(format!("{}.c", source))).expect("Failed to prepare source");
}

pub fn prepare_configuration(dst: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets")
        .join("cpkit.toml");
    std::fs::copy(src, dst.join("cpkit.toml")).expect("Failed to prepare configuration");
}

pub fn run_cpkit<I, S>(args: I, workspace: &Path) -> Assert
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::cargo_bin("cpkit")
        .expect("Failed to run cpkit")
        .args(args)
        .current_dir(workspace)
        .assert()
}

pub fn run_artifact(name: &str, workspace: &Path) -> Assert {
    Command::new(workspace.join(name)).assert()
}
