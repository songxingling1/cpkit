use anyhow::{Context, Result, ensure};
use std::{ffi::OsString, path::PathBuf};

/// 产物构建器，用于从源码路径生成一个产物路径。
#[derive(bon::Builder)]
#[builder(finish_fn(vis = "", name = build_internal))]
pub struct Artifact {
    #[builder(default = Vec::new())]
    options: Vec<OsString>,
    source: Vec<PathBuf>,
    #[cfg_attr(unix, builder(default = PathBuf::from("a.out")))]
    #[cfg_attr(windows, builder(default = PathBuf::from("a.exe")))]
    output: PathBuf,
    compiler: PathBuf,
}

/// 产物构建错误
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    /// 源码路径不合法
    #[error("The source file should exist and be a file.")]
    InvalidSource,
    /// 产物路径不合法
    #[error("Artifact should't be a directory.")]
    InvalidArtifact,
    /// 编译错误
    #[error("{0}")]
    CompilationFailed(String),
    /// 产物在编译之后仍然不存在
    #[error("The artifact still does not exist after compilation.")]
    ArtifactNotExist,
}

impl<S: artifact_builder::IsComplete> ArtifactBuilder<S> {
    /// 编译
    ///
    /// # 参数
    /// - `runner`: 自定义编译器运行器，方便 Mock。
    async fn compile_with<R>(self, runner: R) -> Result<PathBuf>
    where
        R: AsyncFnOnce(&mut tokio::process::Command) -> tokio::io::Result<std::process::Output>,
    {
        let this = self.build_internal();
        ensure!(
            this.source.iter().all(|p| p.is_file()),
            ArtifactError::InvalidSource
        );
        ensure!(!this.output.is_dir(), ArtifactError::InvalidArtifact);
        let mut tmp = tokio::process::Command::new(this.compiler);
        let cmd = tmp
            .args(this.source)
            .arg("-o")
            .arg(&this.output)
            .args(this.options);
        let res = runner(cmd).await.context("Failed to start the compiler.")?;
        ensure!(
            res.status.success(),
            ArtifactError::CompilationFailed(vecu8_to_string_lossy(res.stderr))
        );
        ensure!(this.output.is_file(), ArtifactError::ArtifactNotExist);
        Ok(this.output)
    }
    /// 编译得到产物。
    pub async fn compile(self) -> Result<PathBuf> {
        self.compile_with(async |cmd| cmd.output().await).await
    }
}

#[must_use]
pub fn vecu8_to_string_lossy(vec: Vec<u8>) -> String {
    match String::from_utf8(vec) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    }
}

#[cfg(test)]
#[path = "tests/test_compile.rs"]
mod tests;
