use std::{path::PathBuf, time::Duration};

#[derive(Debug, bon::Builder)]
pub struct Task {
    pub(crate) id: u32,
    pub(crate) executable: PathBuf,
    pub(crate) time_limit: Option<Duration>,
}

pub struct TaskResult {
    /// 原始任务编号
    pub task: u32,
    /// 资源占用
    pub usage: Usage,
}

pub struct Usage {
    /// 运行时间占用
    pub time: Duration,
    /// 内存占用（MiB）
    pub memory: f64,
    /// 退出状态
    pub status: crate::status::PrintableStatus,
}

#[derive(thiserror::Error, Debug)]
pub enum TaskError {
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Unable to connect to the processor")]
    SpawnNotConnected,
    #[error("Unable to connect to the processor")]
    RecvNotConnected,
    #[error("The processor is shutting down")]
    Shutdown,
}
