use std::path::Path;

use anyhow::{Context, Result};

use crate::{
    cli::CompileCommand, compile::ArtifactBuilder, configuration::Config, print::print_progress,
};

/// 承接编译请求。
pub fn compile(config: &Config, CompileCommand { source, output }: CompileCommand) -> Result<()> {
    print_progress("Compiling", &source.to_string_lossy());
    let mut tmp = ArtifactBuilder::new(&source, Path::new(&config.compile.compiler));
    if let Some(output) = output {
        tmp = tmp.artifact(&output);
    }
    let res = tmp
        .args(&config.compile.flag)
        .compile()
        .context("Compilation failed.")?;
    print_progress(
        "Finished",
        &format!(
            "Successfully compiled {} to {}.",
            source.to_string_lossy(),
            res.to_string_lossy()
        ),
    );
    Ok(())
}
