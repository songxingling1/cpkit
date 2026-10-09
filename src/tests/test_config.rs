use super::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct TestConfig {
    foo: String,
    bar: String,
}
#[test]
fn success_discover() {
    let dir = tempfile::TempDir::new().expect("Failed to create temporary directory.");
    let inner = dir
        .path()
        .join("foo")
        .join("bar")
        .join("a")
        .join("b")
        .join("c");
    std::fs::create_dir_all(&inner).expect("Failed to create temporary directory.");
    let config = tempfile::NamedTempFile::new_in(&dir).expect("Failed to crate temporary file.");
    let res = discover_workspace(
        &inner,
        config
            .path()
            .file_name()
            .expect("Failed to get filename")
            .to_str()
            .expect("Failed to get filename"),
    )
    .expect("A valid configuration should succeed");
    assert_eq!(res, dir.path());
}
#[test]
fn failed_discover() {
    let dir = tempfile::TempDir::new().expect("Failed to create temporary directory.");
    let res = discover_workspace(dir.path(), "config-doent-exist");
    assert!(res.is_none(), "A invalid configuration should fail");
}
#[tokio::test]
async fn succeed_parse_file() {
    let config = TestConfig {
        foo: String::from("foo"),
        bar: String::from("bar"),
    };
    let str = toml::to_string_pretty(&config).expect("Failed to create a test config");
    let mut file = tempfile::NamedTempFile::new().expect("Failed to create temporary file");
    write!(file, "{}", str).expect("Failed to write config to temporary file");
    let res = load_config(file.path())
        .await
        .expect("A valid config should succeed.");
    assert_eq!(config, res);
}
#[tokio::test]
async fn failed_parse_path() {
    let err = load_config::<TestConfig>(Path::new(""))
        .await
        .expect_err("A invalid config should fail");
    assert!(matches!(err, ConfigError::IO(_)));
    let ConfigError::IO(e) = err else {
        panic!("A not found config should return IO error.")
    };
    assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn success_find_problem_config() {
    let dir = tempfile::TempDir::new().expect("Failed to create temporary directory");
    let res = dir.path().join(".cpkit").join("test.cpp");
    std::fs::create_dir_all(&res).expect("Failed to create temporary directory");
    let ans = get_problem_config_dir(&dir.path().join("test.cpp"), dir.path(), ".cpkit")
        .expect("A valid path should succeed");
    assert_eq!(ans, res);
}

#[test]
fn invalid_arg_problem_config() {
    let dir = tempfile::TempDir::new().expect("Failed to create temporary directory");
    let res = dir.path().join(".cpkit").join("test.cpp");
    std::fs::create_dir_all(&res).expect("Failed to create temporary directory");
    let ans = get_problem_config_dir(
        &dir.path().join("test.cpp"),
        &dir.path().join("invalid"),
        ".cpkit",
    );
    assert!(ans.is_none());
}

#[test]
fn problem_config_not_found() {
    let dir = tempfile::TempDir::new().expect("Failed to create temporary directory");
    let ans = get_problem_config_dir(&dir.path().join("test.cpp"), dir.path(), ".cpkit");
    assert!(ans.is_none());
}
