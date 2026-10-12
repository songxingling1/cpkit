use std::path::{Component, Path, PathBuf};

use directories::ProjectDirs;
use sha2::Digest;
use tokio::io::AsyncWriteExt;

use crate::configuration::{Config, ProblemConfig, ProblemType};

#[derive(thiserror::Error, Debug)]
pub enum ContextError {
    #[error("Workspace not found.")]
    WorkspaceNotFound,
    #[error("Configuration not found.")]
    ConfigurationNotFound,
    #[error("Failed to read configuration: {0}.")]
    ReadConfigurationIO(#[source] std::io::Error),
    #[error("Failed to parse configuration: {0}.")]
    ReadConfigurationParse(#[source] toml::de::Error),
    #[error("Failed to write configuration: {0}.")]
    WriteConfigurationIO(#[source] std::io::Error),
    #[error("Failed to serialize configuration: {0}.")]
    WriteConfigurationSerialize(#[source] toml::ser::Error),
    #[error("Invalid input: {0}. (The file may no longer be in the current workspace.)")]
    InvalidInput(PathBuf),
    #[error("Failed to calc file hash: {0}.")]
    HashIO(#[source] std::io::Error),
}

/// 全局程序上下文，存储各种配置
pub struct Context {
    current: PathBuf,
    workspace: PathBuf,
    project_dir: ProjectDirs,
    global: Option<Config>,
}

pub struct ProblemContext {
    problem: PathBuf,
    config: PathBuf,
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
        let current = std::env::current_dir().map_err(ContextError::ReadConfigurationIO)?;
        let workspace =
            Self::discover_workspace(&current, ".cpkit").ok_or(ContextError::WorkspaceNotFound)?;
        let project_dir = ProjectDirs::from("", "supermasarada", "cpkit")
            .ok_or(ContextError::ConfigurationNotFound)?;
        Ok(Context {
            current,
            workspace,
            project_dir,
            global: None,
        })
    }
    /// 加载全局配置（在已经存在的时候返回 Ok(())）
    pub async fn load_config(&mut self) -> Result<(), ContextError> {
        if self.global.is_some() {
            return Ok(());
        }
        let cfg = self.project_dir.config_dir().join("configuration.toml");
        let content = match tokio::fs::read_to_string(&cfg).await {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(ContextError::ConfigurationNotFound);
            }
            Err(err) => {
                return Err(ContextError::ReadConfigurationIO(err));
            }
        };
        let config: Config =
            toml::from_str(&content).map_err(ContextError::ReadConfigurationParse)?;
        self.global = Some(config);
        Ok(())
    }
    /// 将用户输入的题目源码路径变成正确的，相对于工作目录的路径
    fn get_problem_token(&self, problem: &Path) -> Result<PathBuf, ContextError> {
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
            .map_err(|_| ContextError::InvalidInput(problem.to_owned()))
            .map(std::borrow::ToOwned::to_owned)
    }
    /// 获取全局配置
    pub fn get_config(&self) -> Option<&Config> {
        self.global.as_ref()
    }
    /// 获取缓存路径
    pub fn get_cache_dir(&self) -> &Path {
        self.project_dir.cache_dir()
    }
    /// 获取题目上下文
    pub fn get_problem_context(&self, problem: &Path) -> Result<ProblemContext, ContextError> {
        let config = self
            .workspace
            .join(".cpkit")
            .join(&problem)
            .join("config.toml");
        let problem = self.workspace.join(self.get_problem_token(problem)?);
        Ok(ProblemContext {
            problem,
            config,
            problem_config: None,
        })
    }
}

impl ProblemContext {
    /// 加载题目配置（已经存在返回 Ok(())）
    pub async fn load_config(&mut self) -> Result<(), ContextError> {
        if self.problem_config.is_some() {
            return Ok(());
        }
        let content = match tokio::fs::read_to_string(&self.config).await {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(ContextError::ConfigurationNotFound);
            }
            Err(err) => {
                return Err(ContextError::ReadConfigurationIO(err));
            }
        };
        let config: ProblemConfig =
            toml::from_str(&content).map_err(ContextError::ReadConfigurationParse)?;
        self.problem_config = Some(config);
        Ok(())
    }
    /// 获取题目配置
    #[allow(dead_code)]
    pub fn get_config(&self) -> Option<&ProblemConfig> {
        self.problem_config.as_ref()
    }
    /// 获取编译题目的源文件需要的源码列表，在没有加载题目配置的时候返回 `None`
    pub fn get_source_list(&self) -> Option<Vec<PathBuf>> {
        match &self.problem_config.as_ref()?.problem_type {
            ProblemType::Tranditional { name: _ } => Some(vec![self.problem.clone()]),
            ProblemType::Interactive { header: _, grader } => Some(vec![
                self.problem.clone(),
                self.config.parent().unwrap().join(grader), // config 只能通过 Context 构造，保证了一定有父目录
            ]),
        }
    }
    /// 存储题目配置
    pub async fn write_problem_config(&self, config: &ProblemConfig) -> Result<(), ContextError> {
        tokio::fs::create_dir_all(self.config.parent().unwrap()) // config 只能通过 Context 构造，保证了一定有父目录
            .await
            .map_err(ContextError::WriteConfigurationIO)?;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.config)
            .await
            .map_err(ContextError::WriteConfigurationIO)?;
        file.write_all(
            &toml::to_string_pretty(config)
                .map_err(ContextError::WriteConfigurationSerialize)?
                .into_bytes(),
        )
        .await
        .map_err(ContextError::WriteConfigurationIO)?;
        Ok(())
    }
    pub async fn get_hash(&self) -> Result<String, ContextError> {
        let content = tokio::fs::read(&self.problem)
            .await
            .map_err(ContextError::HashIO)?;
        Ok(hex::encode(sha2::Sha256::digest(content)))
    }
}
