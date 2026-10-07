use anyhow::{Context, Result, ensure};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::print::vecu8_to_string_lossy;

/// 产物构建器，用于从源码路径生成一个产物路径。
pub struct ArtifactBuilder {
    source: PathBuf,
    artifact: PathBuf,
    options: Vec<OsString>,
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

impl ArtifactBuilder {
    /// 指定源码和编译器，生成一个产物构建器。
    /// 默认使用 `a.out` 作为产物名称，生成在当前工作目录。
    /// 默认没有编译选项。
    pub fn new(source: &Path, compiler: &Path) -> ArtifactBuilder {
        ArtifactBuilder {
            source: source.to_owned(),
            artifact: PathBuf::from("a.out"),
            options: Vec::new(),
            compiler: compiler.to_owned(),
        }
    }
    /// 附加一个编译选项。
    #[allow(dead_code)]
    pub fn arg<O: AsRef<OsStr>>(mut self, option: O) -> Self {
        self.options.push(option.as_ref().to_owned());
        self
    }
    /// 附加多个编译选项。
    pub fn args<I, O>(mut self, option: I) -> Self
    where
        I: IntoIterator<Item = O>,
        O: AsRef<OsStr>,
    {
        self.options
            .extend(option.into_iter().map(|o| o.as_ref().to_owned()));
        self
    }
    /// 指定生成物位置。
    pub fn artifact(mut self, artifact: &Path) -> Self {
        artifact.clone_into(&mut self.artifact);
        self
    }
    /// 编译
    ///
    /// # 参数
    /// - `runner`: 自定义编译器运行器，方便 Mock。
    async fn compile_with<R>(self, runner: R) -> Result<PathBuf>
    where
        R: AsyncFnOnce(&mut tokio::process::Command) -> tokio::io::Result<std::process::Output>,
    {
        ensure!(self.source.is_file(), ArtifactError::InvalidSource);
        ensure!(!self.artifact.is_dir(), ArtifactError::InvalidArtifact);
        let mut tmp = tokio::process::Command::new(self.compiler);
        let cmd = tmp
            .arg(self.source)
            .arg("-o")
            .arg(&self.artifact)
            .args(self.options);
        let res = runner(cmd).await.context("Failed to start the compiler.")?;
        ensure!(
            res.status.success(),
            ArtifactError::CompilationFailed(vecu8_to_string_lossy(res.stderr))
        );
        ensure!(self.artifact.is_file(), ArtifactError::ArtifactNotExist);
        Ok(self.artifact)
    }
    /// 编译得到产物。
    pub async fn compile(self) -> Result<PathBuf> {
        self.compile_with(async |cmd| cmd.output().await).await
    }
}

#[cfg(test)]
#[path = "tests/test_compile.rs"]
mod tests;
