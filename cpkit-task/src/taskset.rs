use std::thread;

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use tokio_util::sync::CancellationToken;

use crate::task::{Task, TaskError, TaskResult};

pub struct TaskSet {
    requests: UnboundedSender<(Task, CancellationToken)>,
    results: UnboundedReceiver<Result<TaskResult, TaskError>>,
    cancel: CancellationToken,
}

impl TaskSet {
    /// 创建新的任务集合。
    ///
    /// # Errors
    /// 当发生 IO 错误时返回 `io::Error`。
    #[must_use]
    pub fn new() -> Self {
        let (req_tx, req_rx) = unbounded();
        let (res_tx, res_rx) = unbounded();
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        #[cfg(target_os = "linux")]
        thread::spawn(move || {
            async_io::block_on(crate::imp::process(req_rx, res_tx, worker_cancel));
        });
        Self {
            requests: req_tx,
            results: res_rx,
            cancel,
        }
    }
    /// 拉起一个新的任务。
    ///
    /// # Errors
    /// 当内部工作线程死亡时，返回 `SpawnNotConnected`。
    pub fn spawn(&self, task: Task) -> Result<CancellationToken, TaskError> {
        let token = self.cancel.child_token();
        self.requests
            .unbounded_send((task, token.clone()))
            .map_err(|_| TaskError::SpawnNotConnected)?;
        Ok(token)
    }
    /// 取得一个结束任务的结果。
    ///
    /// # Errors
    /// 当内部工作线程死亡时，返回 `RecvNotConnected`。
    pub async fn next(&mut self) -> Result<TaskResult, TaskError> {
        self.results
            .recv()
            .await
            .map_err(|_| TaskError::RecvNotConnected)?
    }
}

impl Drop for TaskSet {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

impl Default for TaskSet {
    fn default() -> Self {
        Self::new()
    }
}
