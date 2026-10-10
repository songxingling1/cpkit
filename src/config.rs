use crate::{
    configuration::{Config, ProblemConfig, ProblemType},
    utils::calc_file_hash,
};
use directories::ProjectDirs;
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncWriteExt;

/// 上下文错误
#[derive(thiserror::Error, Debug)]
pub enum ContextError {
    #[error("IO Error: {0}.")]
    IO(std::io::Error),
    #[error("Parse Error: {0}.")]
    Parse(toml::de::Error),
    #[error("Serialize Error: {0}.")]
    Serialize(toml::ser::Error),
    #[error("{0} is not found.")]
    NotFound(&'static str),
    #[error("{0} is already exists.")]
    AlreadyExists(&'static str),
    #[error("{0} is invalid.")]
    Invalid(&'static str),
}

/// 程序上下文，存储各种配置
pub struct Context {
    current: PathBuf,
    workspace: PathBuf,
    problem: PathBuf,
    project_dir: ProjectDirs,
    global: Option<Config>,
    problem_config: Option<ProblemConfig>,
}

impl Context {
    /// 寻找工作区
    fn discover_workspace(dir: &Path, name: &str) -> Option<PathBuf> {
        let mut cur = dir.to_owned();
        loop {
            if cur.join(name).exists() {
                return Some(cur);
            }
            if !cur.pop() {
                return None;
            }
        }
    }
    /// 创建一个新的上下文，自动寻找最近的工作区。
    pub fn new() -> Result<Context, ContextError> {
        let current = std::env::current_dir().map_err(ContextError::IO)?;
        let workspace = Self::discover_workspace(&current, ".cpkit")
            .ok_or(ContextError::NotFound("Workspace"))?;
        let project_dir = ProjectDirs::from("", "supermasarada", "cpkit")
            .ok_or(ContextError::NotFound("Config directory"))?;
        Ok(Context {
            current,
            workspace,
            problem: PathBuf::new(),
            project_dir,
            global: None,
            problem_config: None,
        })
    }
    /// 加载全局配置
    pub async fn load_global_config(&mut self) -> Result<(), ContextError> {
        if self.global.is_some() {
            return Err(ContextError::AlreadyExists("Configuration"));
        }
        let cfg = self.project_dir.config_dir().join("configuration.toml");
        let content = match tokio::fs::read_to_string(&cfg).await {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(ContextError::NotFound("Configuration"));
            }
            Err(err) => {
                return Err(ContextError::IO(err));
            }
        };
        let config: Config = toml::from_str(&content).map_err(ContextError::Parse)?;
        self.global = Some(config);
        Ok(())
    }
    /// 加载题目配置
    pub async fn load_problem_config(&mut self, problem: &Path) -> Result<(), ContextError> {
        if self.problem_config.is_some() {
            return Err(ContextError::AlreadyExists("Problem config"));
        }
        let cfg = self
            .workspace
            .join(".cpkit")
            .join(problem)
            .join("config.toml");
        let content = match tokio::fs::read_to_string(&cfg).await {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(ContextError::NotFound("Problem config"));
            }
            Err(err) => {
                return Err(ContextError::IO(err));
            }
        };
        let config: ProblemConfig = toml::from_str(&content).map_err(ContextError::Parse)?;
        problem.clone_into(&mut self.problem);
        self.problem_config = Some(config);
        Ok(())
    }
    /// 将用户输入的题目源码路径变成正确的，相对于工作目录的路径
    pub fn get_problem_token(&self, problem: &Path) -> Result<PathBuf, ContextError> {
        let res = if problem.is_absolute() {
            problem.to_owned()
        } else {
            let mut res = self.current.clone();
            for component in problem.components() {
                match component {
                    Component::Prefix(_) | Component::RootDir => {
                        problem.clone_into(&mut res);
                        break;
                    }
                    Component::CurDir => {}
                    Component::ParentDir => {
                        res.pop();
                    }
                    Component::Normal(p) => {
                        res.push(p);
                    }
                }
            }
            res
        };
        res.strip_prefix(&self.workspace)
            .map_err(|_| ContextError::Invalid("The source path"))
            .map(std::borrow::ToOwned::to_owned)
    }
    /// 获取题目配置
    #[allow(dead_code)]
    pub fn get_problem_config(&self) -> Option<&ProblemConfig> {
        self.problem_config.as_ref()
    }
    /// 获取全局配置
    pub fn get_global_config(&self) -> Option<&Config> {
        self.global.as_ref()
    }
    /// 获取编译题目的源文件需要的源码列表，在没有加载题目配置的时候返回 `None`
    pub fn get_source_list(&self) -> Option<Vec<PathBuf>> {
        match &self.problem_config.as_ref()?.problem_type {
            ProblemType::Tranditional { name: _ } => Some(vec![self.workspace.join(&self.problem)]),
            ProblemType::Interactive { header: _, grader } => Some(vec![
                self.workspace.join(&self.problem),
                self.workspace
                    .join(".cpkit")
                    .join(&self.problem)
                    .join(grader),
            ]),
        }
    }
    /// 获取当前题目可执行文件的输出目录
    pub async fn get_output_path(&self) -> Result<PathBuf, ContextError> {
        let hash = calc_file_hash(&self.problem)
            .await
            .map_err(ContextError::IO)?;
        let cache = self.project_dir.cache_dir();
        tokio::fs::create_dir_all(cache)
            .await
            .map_err(ContextError::IO)?;
        Ok(cache.join(hash))
    }
    /// 存储题目配置
    pub async fn write_problem_config(
        &self,
        problem: &Path,
        config: &ProblemConfig,
    ) -> Result<(), ContextError> {
        let cfg = self.workspace.join(".cpkit").join(problem);
        tokio::fs::create_dir_all(&cfg)
            .await
            .map_err(ContextError::IO)?;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(cfg.join("config.toml"))
            .await
            .map_err(ContextError::IO)?;
        file.write_all(
            &toml::to_string_pretty(config)
                .map_err(ContextError::Serialize)?
                .into_bytes(),
        )
        .await
        .map_err(ContextError::IO)?;
        Ok(())
    }
}

// #[cfg(test)]
// #[path = "tests/test_config.rs"]
// mod test;
