use serde::Deserialize;
use std::path::PathBuf;
use crate::config::LoadConfig;

#[derive(Deserialize)]
pub struct CompileConfig {
    compiler: PathBuf,
    flag: Vec<String>,
}

#[derive(Deserialize)]
pub struct Config {
    compile: CompileConfig,
}

impl LoadConfig for Config {}
