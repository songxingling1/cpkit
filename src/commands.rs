use std::path::Path;

use anyhow::{Context, Result};

use crate::{cli::CompileCommand, compile::ArtifactBuilder, configuration::Config, print_progress};

/// 承接编译请求。
pub async fn compile(
    config: &Config,
    CompileCommand { source, output }: CompileCommand,
) -> Result<()> {
    print_progress!("Compiling", "{}", source.display());
    let mut tmp = ArtifactBuilder::new(&source, Path::new(&config.compile.compiler));
    if let Some(output) = output {
        tmp = tmp.artifact(&output);
    }
    let res = tmp
        .args(&config.compile.flag)
        .compile()
        .await
        .context("Compilation failed.")?;
    print_progress!(
        "Finished",
        "Successfully compiled {} to {}.",
        source.display(),
        res.display()
    );
    Ok(())
}
