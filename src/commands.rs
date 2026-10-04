use std::path::Path;

use anyhow::{Context, Result};

use crate::{compile::ArtifactBuilder, print::print_progress};

pub fn compile(source: &Path, output: Option<&Path>) -> Result<()> {
    print_progress("Compiling", &source.to_string_lossy());
    let mut tmp = ArtifactBuilder::new(source, Path::new("g++"));
    if let Some(output) = output {
        tmp = tmp.artifact(output);
    }
    let res = tmp
        .args(["-O2", "-std=c++14", "-Wall"])
        .arg("-fdiagnostics-color=always")
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
