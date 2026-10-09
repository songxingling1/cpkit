#![allow(dead_code)]
use assert_cmd::{Command, assert::Assert, cargo::cargo_bin};
use std::{ffi::OsStr, path::Path};

pub fn init_workspace() -> tempfile::TempDir {
    tempfile::TempDir::new().expect("Failed to init temporary directory")
}

pub fn prepare_source(source: &str, dst: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets")
        .join(format!("{}.c", source));
    let cfg = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets")
        .join(format!("{}.toml", source));
    let cfgdst = dst.join(".cpkit").join(format!("{}.c", source));
    std::fs::create_dir_all(&cfgdst).expect("Failed to prepare source");
    std::fs::copy(&src, dst.join(format!("{}.c", source))).expect("Failed to prepare source");
    std::fs::copy(cfg, cfgdst.join("config.toml")).expect("Failed to prepare source");
    println!("{} {}", src.display(), cfgdst.display());
}

pub fn prepare_source_without_cfg(source: &str, dst: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets")
        .join(format!("{}.c", source));
    std::fs::copy(&src, dst.join(format!("{}.c", source))).expect("Failed to prepare source");
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

pub fn run_cpkit_with_child<I, S>(args: I, workspace: &Path) -> std::process::Child
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    std::process::Command::new(cargo_bin("cpkit"))
        .args(args)
        .current_dir(workspace)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to run cpkit")
}

pub fn run_artifact(name: &str, workspace: &Path) -> Assert {
    Command::new(workspace.join(name)).assert()
}
