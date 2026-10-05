use super::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct TestConfig {
    foo: String,
    bar: String,
}
impl LoadConfig for TestConfig {}
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
    let config =
        tempfile::NamedTempFile::new_in(&dir).expect("Failed to crate temporary file.");
    let res = discover_configure(
        &inner,
        config
            .path()
            .file_name()
            .expect("Failed to get filename")
            .to_str()
            .expect("Failed to get filename"),
    )
    .expect("A valid configuration should succeed");
    assert_eq!(res, config.path());
}
#[test]
fn failed_discover() {
    let dir = tempfile::TempDir::new().expect("Failed to create temporary directory.");
    let res = discover_configure(dir.path(), "config-doent-exist");
    assert!(res.is_none(), "A invalid configuration should fail");
}
#[test]
fn succeed_parse_str() {
    let config = TestConfig {
        foo: String::from("foo"),
        bar: String::from("bar"),
    };
    let str = toml::to_string_pretty(&config).expect("Failed to create a test config");
    let res = TestConfig::from_str(&str).expect("A valid config should succeed.");
    assert_eq!(config, res);
}
#[test]
fn succeed_parse_file() {
    let config = TestConfig {
        foo: String::from("foo"),
        bar: String::from("bar"),
    };
    let str = toml::to_string_pretty(&config).expect("Failed to create a test config");
    let mut file = tempfile::NamedTempFile::new().expect("Failed to create temporary file");
    write!(file, "{}", str).expect("Failed to write config to temporary file");
    let res = TestConfig::from_path(file.path()).expect("A valid config should succeed.");
    assert_eq!(config, res);
}
#[test]
fn succeed_parse_dir() {
    let config = TestConfig {
        foo: String::from("foo"),
        bar: String::from("bar"),
    };
    let str = toml::to_string_pretty(&config).expect("Failed to create a test config");
    let dir = tempfile::TempDir::new().expect("Failed to create temporary directory");
    let mut file =
        tempfile::NamedTempFile::new_in(&dir).expect("Failed to create temporary file");
    write!(file, "{}", str).expect("Failed to write config to temporary file");
    let res = TestConfig::from_dir(
        dir.path(),
        file.path()
            .file_name()
            .expect("Failed to get filename")
            .to_str()
            .expect("Failed to get filename"),
    )
    .expect("A valid config should succeed.");
    assert_eq!(config, res);
}
#[test]
fn failed_parse_str() {
    let err = TestConfig::from_str("").expect_err("A invalid config should fail");
    assert!(matches!(err, ConfigError::Parse(_)));
}
#[test]
fn failed_parse_path() {
    let err = TestConfig::from_path(Path::new("")).expect_err("A invalid config should fail");
    assert!(matches!(err, ConfigError::IO(_)));
    let ConfigError::IO(e) = err else {
        panic!("A not found config should return IO error.")
    };
    assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
}
#[test]
fn failed_parse_dir() {
    let err = TestConfig::from_dir(Path::new("/not/found"), "config")
        .expect_err("A invalid config should fail");
    assert!(matches!(err, ConfigError::IO(_)));
    let ConfigError::IO(e) = err else {
        panic!("A not found config should return IO error.")
    };
    assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
}