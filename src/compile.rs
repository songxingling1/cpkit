use anyhow::{Context, Result, ensure};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

pub struct ArtifactBuilder {
    source: PathBuf,
    artifact: PathBuf,
    options: Vec<OsString>,
    compiler: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("The source file should exist and be a file.")]
    InvalidSource,
    #[error("Artifact should't be a directory.")]
    InvalidArtifact,
    #[error("{0}")]
    CompilationFailed(String),
    #[error("The artifact still does not exist after compilation.")]
    ArtifactNotExist,
}

impl ArtifactBuilder {
    pub fn new(source: &Path, compiler: &Path) -> ArtifactBuilder {
        ArtifactBuilder {
            source: source.to_owned(),
            artifact: PathBuf::from("a.out"),
            options: Vec::new(),
            compiler: compiler.to_owned(),
        }
    }
    #[allow(dead_code)]
    pub fn arg<O: AsRef<OsStr>>(mut self, option: O) -> Self {
        self.options.push(option.as_ref().to_owned());
        self
    }
    pub fn args<I, O>(mut self, option: I) -> Self
    where
        I: IntoIterator<Item = O>,
        O: AsRef<OsStr>,
    {
        self.options
            .extend(option.into_iter().map(|o| o.as_ref().to_owned()));
        self
    }
    pub fn artifact(mut self, artifact: &Path) -> Self {
        self.artifact = artifact.to_owned();
        self
    }
    fn compile_with<R>(self, runner: R) -> Result<PathBuf>
    where
        R: FnOnce(&mut std::process::Command) -> std::io::Result<std::process::Output>,
    {
        ensure!(self.source.is_file(), ArtifactError::InvalidSource);
        ensure!(!self.artifact.is_dir(), ArtifactError::InvalidArtifact);
        let mut tmp = std::process::Command::new(self.compiler);
        let cmd = tmp
            .arg(self.source)
            .arg("-o")
            .arg(&self.artifact)
            .args(self.options);
        let res = runner(cmd).context("Failed to start the compiler.")?;
        ensure!(
            res.status.success(),
            ArtifactError::CompilationFailed(String::from_utf8_lossy(&res.stderr).to_string())
        );
        ensure!(self.artifact.is_file(), ArtifactError::ArtifactNotExist);
        Ok(self.artifact)
    }
    pub fn compile(self) -> Result<PathBuf> {
        self.compile_with(|cmd| cmd.output())
    }
}

#[cfg(test)]
#[path = "tests/test_compile.rs"]
mod tests;
