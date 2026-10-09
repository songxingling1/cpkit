use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 编译相关配置
#[derive(Deserialize)]
pub struct CompileConfig {
    /// 编译器
    pub compiler: PathBuf,
    /// 编译选项
    pub flag: Vec<String>,
}

/// CLI 配置
#[derive(Deserialize, Default)]
pub struct Config {
    pub compile: CompileConfig,
}

impl Default for CompileConfig {
    fn default() -> Self {
        CompileConfig {
            compiler: PathBuf::from("g++"),
            flag: vec!["-fdiagnostics-color=always".to_owned()],
        }
    }
}

/// 题目类型
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum ProblemType {
    Tranditional { name: String },
    Interactive { header: PathBuf, grader: PathBuf },
}

/// 比较器
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Checker {
    #[default]
    Builtin,
    Special {
        path: PathBuf,
    },
}

/// 测试用例
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Testcase {
    pub input: PathBuf,
    pub output: PathBuf,
}

/// 题目配置
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProblemConfig {
    /// 内存限制（MiB）
    pub memory_limit: u32,
    /// 时间限制（毫秒）
    pub time_limit: u32,
    /// 题目类型
    pub problem_type: ProblemType,
    /// 测试用例
    pub testcase: Vec<Testcase>,
    /// 比较器
    pub checker: Checker,
}

impl Default for ProblemType {
    fn default() -> Self {
        ProblemType::Tranditional {
            name: String::new(),
        }
    }
}

impl Default for ProblemConfig {
    fn default() -> Self {
        ProblemConfig {
            memory_limit: 512,
            time_limit: 1024,
            problem_type: ProblemType::default(),
            testcase: vec![],
            checker: Checker::default(),
        }
    }
}
