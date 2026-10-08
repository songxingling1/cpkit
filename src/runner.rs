use crate::rusage::{ExitInformation, RusageExt};
use std::{process::Command, time::Duration};
use tokio::time::timeout;

pub async fn run_with_time_limit(
    cmd: &mut Command,
    limit: Duration,
) -> std::io::Result<ExitInformation> {
    let mut child = cmd.spawn_with_rusage()?;
    match timeout(limit, child.wait()).await {
        Ok(Ok(i)) => Ok(i),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            child.kill()?;
            child.wait().await
        }
    }
}
