#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{FutureExt, StreamExt, select, stream::FuturesUnordered};
use tokio_util::sync::CancellationToken;

use crate::{Task, TaskResult, task::TaskError};

pub async fn process(
    mut request: UnboundedReceiver<(Task, CancellationToken)>,
    result: UnboundedSender<Result<TaskResult, TaskError>>,
    cancel: CancellationToken,
) {
    let mut futs = FuturesUnordered::new();
    loop {
        select! {
            req = request.recv().fuse() => {
                let Ok((task, token)) = req else {
                    break;
                };
                #[cfg(target_os = "linux")]
                futs.push(linux::run_task(task, token));
                #[cfg(target_os = "windows")]
                futs.push(windows::run_task(task, token));

            },
            res = futs.select_next_some().fuse() => {
                let _ = result.unbounded_send(res);
            },
            () = cancel.cancelled().fuse() => {
                while let Some(res) = futs.next().await { let _ = result.unbounded_send(res); }
                break;
            }
        }
    }
}
