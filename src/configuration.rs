use crate::config::LoadConfig;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct CompileConfig {
    pub compiler: PathBuf,
    pub flag: Vec<String>,
}

#[derive(Deserialize)]
pub struct Config {
    pub compile: CompileConfig,
}

impl LoadConfig for Config {}
impl Default for CompileConfig {
    fn default() -> Self {
        CompileConfig {
            compiler: PathBuf::from("g++"),
            flag: vec!["-fdiagnostics-color=always".to_owned()],
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Config {
            compile: CompileConfig::default(),
        }
    }
}
