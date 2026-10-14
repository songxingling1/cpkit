
use super::{ExitInformation, RunnerExt};
use std::time::Duration;

fn command(script: &str) -> tokio::process::Command {
    #[cfg(unix)]
    {
        let mut command = tokio::process::Command::new("sh");
        command.args(["-c", script]);
        command
    }
    #[cfg(windows)]
    {
        let mut command = tokio::process::Command::new("cmd");
        command.args(["/C", script]);
        command
    }
}

fn assert_exit_information(info: &ExitInformation) {
    assert!(info.time_usage < Duration::from_secs(60));
    assert!(info.memory_usage.is_finite());
    assert!(info.memory_usage >= 0.0);
    assert!(!info.status.is_empty());
}

#[tokio::test]
async fn wait_returns_and_caches_exit_information() {
    let mut command = command("exit 7");
    let mut child = command.spawn_with_runner().expect("child should start");

    let first = child.wait().await.expect("child should exit").clone();
    assert_exit_information(&first);
    #[cfg(unix)]
    assert_eq!(first.status, "None");
    #[cfg(windows)]
    assert!(first.status.contains('7'));

    let second = child
        .wait()
        .await
        .expect("cached result should be returned");
    assert_eq!(second.status, first.status);
    assert_eq!(second.time_usage, first.time_usage);
    assert_eq!(second.memory_usage, first.memory_usage);
}

#[tokio::test]
async fn try_wait_reports_running_then_exited_child() {
    let mut command = command("sleep 0.1; exit 0");
    let mut child = command.spawn_with_runner().expect("child should start");

    assert!(child.try_wait().expect("poll should succeed").is_none());
    let info = child.wait().await.expect("child should exit");
    assert_exit_information(info);
}

#[tokio::test]
async fn kill_stops_child_and_wait_returns_result() {
    let mut command = command("sleep 30");
    let mut child = command.spawn_with_runner().expect("child should start");

    child.kill().expect("child should be killed");
    let info = child.wait().await.expect("killed child should be reaped");
    assert_exit_information(info);
    assert!(info.status.contains("Killed") || info.status.contains("signal"));

    child
        .kill()
        .expect("killing an exited child should be harmless");
}

