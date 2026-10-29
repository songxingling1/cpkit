use super::*;
use std::process::ExitStatus;
#[cfg(unix)]
fn exit_status(code: i32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    // Unix 的 from_raw 接收的是 raw wait status。
    // 正常退出码通常左移 8 位。
    std::process::ExitStatus::from_raw(code << 8)
}
#[cfg(windows)]
pub fn exit_status(code: i32) -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    // Windows 的 from_raw 接收的是直接的退出码 (u32)
    ExitStatus::from_raw(code as u32)
}
fn ok_output() -> tokio::io::Result<std::process::Output> {
    Ok(std::process::Output {
        status: exit_status(0),
        stderr: Vec::new(),
        stdout: Vec::new(),
    })
}
#[tokio::test]
async fn success() {
    let dir = tempfile::TempDir::new().expect("Failed to create a temporary directory.");
    let source = tempfile::NamedTempFile::new_in(&dir)
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let artifact = dir.path().join("artifact");
    let built = Artifact::builder()
        .source(vec![source.to_path_buf()])
        .arg("-O2")
        .args(["-Wall", "-std=c++14"])
        .artifact(artifact.clone())
        .compiler(PathBuf::from("gcc"))
        .compile_with(async |cmd| {
            assert_eq!(cmd.as_std().get_program(), "gcc");
            let args: Vec<OsString> = cmd.as_std().get_args().map(|s| s.to_owned()).collect();
            assert_eq!(args[0], source.as_os_str());
            assert_eq!(args[1].as_os_str(), "-o");
            assert_eq!(args[2], artifact.as_os_str());
            assert_eq!(args[3], "-O2");
            assert_eq!(args[4], "-Wall");
            assert_eq!(args[5], "-std=c++14");
            std::fs::write(&artifact, "artifact").expect("Failed to write artifact");
            ok_output()
        })
        .await
        .expect("A valid compilation should succeed.");
    assert_eq!(built, artifact);
    assert!(artifact.is_file());
}
#[tokio::test]
async fn source_not_found() {
    let err = Artifact::builder()
        .source(vec![PathBuf::from("/the/source/file/not/found")])
        .compiler(PathBuf::from("gcc"))
        .compile_with(async |_| ok_output())
        .await
        .expect_err("A not found file should fail");
    assert!(matches!(
        err.downcast_ref::<ArtifactError>()
            .expect("Failed to extract error"),
        ArtifactError::InvalidSource
    ));
}
#[tokio::test]
async fn source_is_a_directory() {
    let source = tempfile::TempDir::new().expect("Failed to create a temporary directory.");
    let err = Artifact::builder()
        .source(vec![source.path().to_owned()])
        .compiler(PathBuf::from("gcc"))
        .compile_with(async |_| ok_output())
        .await
        .expect_err("a directory source should fail");
    assert!(matches!(
        err.downcast_ref::<ArtifactError>()
            .expect("Failed to extract error"),
        ArtifactError::InvalidSource
    ));
}
#[tokio::test]
async fn artifact_is_a_directory() {
    let source = tempfile::NamedTempFile::new()
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let artifact = tempfile::TempDir::new().expect("Failed to create a temporary directory.");
    let err = Artifact::builder()
        .source(vec![source.to_path_buf()])
        .compiler(PathBuf::from("gcc"))
        .artifact(artifact.path().to_owned())
        .compile_with(async |_| ok_output())
        .await
        .expect_err("a directory artifact should fail");
    assert!(matches!(
        err.downcast_ref::<ArtifactError>()
            .expect("Failed to extract error"),
        ArtifactError::InvalidArtifact
    ));
}
#[tokio::test]
async fn failed_to_start_compiler() {
    let source = tempfile::NamedTempFile::new()
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let err = Artifact::builder()
        .source(vec![source.to_path_buf()])
        .compiler(PathBuf::from("gcc"))
        .compile_with(async |_| Err(std::io::Error::other("test")))
        .await
        .expect_err("a invalid compiler should fail");
    assert!(matches!(
        err.downcast_ref::<std::io::Error>()
            .expect("Failed to extract error")
            .kind(),
        std::io::ErrorKind::Other
    ));
}
#[tokio::test]
async fn compilation_failed() {
    let source = tempfile::NamedTempFile::new()
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let err = Artifact::builder()
        .source(vec![source.to_path_buf()])
        .compiler(PathBuf::from("gcc"))
        .compile_with(async |_| {
            Ok(std::process::Output {
                status: exit_status(1),
                stderr: "compile error".as_bytes().to_vec(),
                stdout: Vec::new(),
            })
        })
        .await
        .expect_err("failed compilation should fail");
    assert!(matches!(
        err.downcast_ref::<ArtifactError>()
            .expect("Failed to extract error"),
        ArtifactError::CompilationFailed(_)
    ));
}
#[tokio::test]
async fn artifact_not_found() {
    let dir = tempfile::TempDir::new().expect("Failed to create a temporary directory.");
    let source = tempfile::NamedTempFile::new_in(&dir)
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let artifact = dir.path().join("artifact");
    let err = Artifact::builder()
        .source(vec![source.to_path_buf()])
        .compiler(PathBuf::from("gcc"))
        .artifact(artifact)
        .compile_with(async |_| ok_output())
        .await
        .expect_err("a not found artifact should fail");
    assert!(matches!(
        err.downcast_ref::<ArtifactError>()
            .expect("Failed to extract error"),
        ArtifactError::ArtifactNotExist
    ));
}
#[tokio::test]
async fn double_source() {
    let dir = tempfile::TempDir::new().expect("Failed to create a temporary directory.");
    let source = tempfile::NamedTempFile::new_in(&dir)
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let source2 = tempfile::NamedTempFile::new_in(&dir)
        .expect("Failed to create a temporary source file.")
        .into_temp_path();
    let artifact = dir.path().join("artifact");
    let built = Artifact::builder()
        .source(vec![source.to_path_buf(), source2.to_path_buf()])
        .compiler(PathBuf::from("gcc"))
        .artifact(artifact.clone())
        .compile_with(async |cmd| {
            let args: Vec<OsString> = cmd.as_std().get_args().map(|s| s.to_owned()).collect();
            assert_eq!(args[0], source.as_os_str());
            assert_eq!(args[1], source2.as_os_str());
            std::fs::write(&artifact, "artifact").expect("Failed to write artifact");
            ok_output()
        })
        .await
        .expect("A valid compilation should succeed.");
    assert_eq!(built, artifact);
    assert!(artifact.is_file());
}
