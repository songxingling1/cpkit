use super::*;
use std::process::ExitStatus;
fn exit_status(code: i32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    // Unix 的 from_raw 接收的是 raw wait status。
    // 正常退出码通常左移 8 位。
    std::process::ExitStatus::from_raw(code << 8)
}
fn ok_output() -> std::io::Result<std::process::Output> {
    Ok(std::process::Output {
        status: exit_status(0),
        stderr: Vec::new(),
        stdout: Vec::new(),
    })
}
#[test]
fn success() {
    let dir = tempfile::TempDir::new().expect("Failed to create a temporary directory.");
    let source = tempfile::NamedTempFile::new_in(&dir)
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let artifact = dir.path().join("artifact");
    let built = ArtifactBuilder::new(&source, Path::new("gcc"))
        .arg("-O2")
        .args(["-Wall", "-std=c++14"])
        .artifact(&artifact)
        .compile_with(|cmd| {
            assert_eq!(cmd.get_program(), "gcc");
            let args: Vec<OsString> = cmd.get_args().map(|s| s.to_owned()).collect();
            assert_eq!(args[0], source.as_os_str());
            assert_eq!(args[1].as_os_str(), "-o");
            assert_eq!(args[2], artifact.as_os_str());
            assert_eq!(args[3], "-O2");
            assert_eq!(args[4], "-Wall");
            assert_eq!(args[5], "-std=c++14");
            std::fs::write(&artifact, "artifact").expect("Failed to write artifact");
            ok_output()
        })
        .expect("A valid compilation should succeed.");
    assert_eq!(built, artifact);
    assert!(artifact.is_file());
}
#[test]
fn source_not_found() {
    let err = ArtifactBuilder::new(Path::new("/the/source/file/not/found"), Path::new("gcc"))
        .compile_with(|_| ok_output())
        .expect_err("A not found file should fail");
    assert!(matches!(
        err.downcast_ref::<ArtifactError>()
            .expect("Failed to extract error"),
        ArtifactError::InvalidSource
    ));
}
#[test]
fn source_is_a_directory() {
    let source = tempfile::TempDir::new().expect("Failed to create a temporary directory.");
    let err = ArtifactBuilder::new(source.path(), Path::new("gcc"))
        .compile_with(|_| ok_output())
        .expect_err("a directory source should fail");
    assert!(matches!(
        err.downcast_ref::<ArtifactError>()
            .expect("Failed to extract error"),
        ArtifactError::InvalidSource
    ));
}
#[test]
fn artifact_is_a_directory() {
    let source = tempfile::NamedTempFile::new()
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let artifact = tempfile::TempDir::new().expect("Failed to create a temporary directory.");
    let err = ArtifactBuilder::new(&source, Path::new("gcc"))
        .artifact(artifact.path())
        .compile_with(|_| ok_output())
        .expect_err("a directory artifact should fail");
    assert!(matches!(
        err.downcast_ref::<ArtifactError>()
            .expect("Failed to extract error"),
        ArtifactError::InvalidArtifact
    ));
}
#[test]
fn failed_to_start_compiler() {
    let source = tempfile::NamedTempFile::new()
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let err = ArtifactBuilder::new(&source, Path::new("gcc"))
        .compile_with(|_| Err(std::io::Error::other("test")))
        .expect_err("a invalid compiler should fail");
    assert!(matches!(
        err.downcast_ref::<std::io::Error>()
            .expect("Failed to extract error")
            .kind(),
        std::io::ErrorKind::Other
    ));
}
#[test]
fn compilation_failed() {
    let source = tempfile::NamedTempFile::new()
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let err = ArtifactBuilder::new(&source, Path::new("gcc"))
        .compile_with(|_| {
            Ok(std::process::Output {
                status: exit_status(1),
                stderr: "compile error".as_bytes().to_vec(),
                stdout: Vec::new(),
            })
        })
        .expect_err("failed compilation should fail");
    assert!(matches!(
        err.downcast_ref::<ArtifactError>()
            .expect("Failed to extract error"),
        ArtifactError::CompilationFailed(_)
    ));
}
#[test]
fn artifact_not_found() {
    let dir = tempfile::TempDir::new().expect("Failed to create a temporary directory.");
    let source = tempfile::NamedTempFile::new_in(&dir)
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let artifact = dir.path().join("artifact");
    let err = ArtifactBuilder::new(&source, Path::new("gcc"))
        .artifact(&artifact)
        .compile_with(|_| ok_output())
        .expect_err("a not found artifact should fail");
    assert!(matches!(
        err.downcast_ref::<ArtifactError>()
            .expect("Failed to extract error"),
        ArtifactError::ArtifactNotExist
    ));
}