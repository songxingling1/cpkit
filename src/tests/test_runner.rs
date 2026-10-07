use super::*;
use mockall::predicate::*;
use std::{process::ExitStatus, time::Duration};

/// 生成 Child，标记值是 pid
fn get_child(tag: i32) -> MockChildLike {
    let mut res = MockChildLike::new();
    res.expect_pid().return_const(tag);
    res.expect_stderr().returning(|| None);
    res.expect_stdout().returning(|| None);
    res.expect_stdin().returning(|| None);
    res
}

/// 生成 rusage 结构体，标记值在 ru_maxrss
fn get_rusage(tag: i64) -> libc::rusage {
    let timeval = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    libc::rusage {
        ru_utime: timeval,
        ru_stime: timeval,
        ru_maxrss: tag,
        ru_ixrss: 0,
        ru_idrss: 0,
        ru_isrss: 0,
        ru_minflt: 0,
        ru_majflt: 0,
        ru_nswap: 0,
        ru_inblock: 0,
        ru_oublock: 0,
        ru_msgsnd: 0,
        ru_msgrcv: 0,
        ru_nsignals: 0,
        ru_nvcsw: 0,
        ru_nivcsw: 0,
    }
}

fn get_signal() -> MockSignalSource {
    let mut sig = MockSignalSource::new();
    sig.expect_recv()
        .returning(|| Box::pin(std::future::ready(())));
    sig
}

fn get_no_signal() -> MockSignalSource {
    let mut sig = MockSignalSource::new();
    sig.expect_recv()
        .returning(|| Box::pin(std::future::pending()));
    sig
}

#[tokio::test]
async fn success() {
    let mut os = MockOs::new();
    let child = get_child(114);
    let sig = get_signal();
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(None))
        .with(eq(114), eq(libc::WNOHANG));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(Some((0, get_rusage(514)))))
        .with(eq(114), eq(libc::WNOHANG));
    let mut child = InnerRusageChild::new(child, Duration::from_secs(3), os, sig);
    let res = child.wait().await.expect("A valid child should succeed");
    assert_eq!(res.rusage.ru_maxrss, 514);
    assert!(res.exit_status.success());
}

#[tokio::test]
async fn short_child() {
    let mut os = MockOs::new();
    let child = get_child(514);
    let sig = get_signal();
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(Some((0, get_rusage(114)))))
        .with(eq(514), eq(libc::WNOHANG));
    let mut child = InnerRusageChild::new(child, Duration::from_secs(3), os, sig);
    let res = child.wait().await.expect("A valid child should succeed");
    assert_eq!(res.rusage.ru_maxrss, 114);
    assert!(res.exit_status.success());
}

#[tokio::test(start_paused = true)]
async fn timeout() {
    let mut os = MockOs::new();
    let child = get_child(514);
    let sig = get_no_signal();
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(None))
        .with(eq(514), eq(libc::WNOHANG));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(Some((0, get_rusage(114)))))
        .with(eq(514), eq(0));
    os.expect_kill()
        .times(1)
        .with(eq(514), eq(9))
        .returning(|_, _| Ok(514));
    let mut child = InnerRusageChild::new(child, Duration::from_secs(3), os, sig);
    let res = child.wait().await.expect("A valid child should succeed");
    assert_eq!(res.rusage.ru_maxrss, 114);
    assert!(res.exit_status.success());
    drop(child);
}

#[tokio::test]
async fn test_drop() {
    let mut os = MockOs::new();
    let child = get_child(514);
    let sig = get_signal();
    os.expect_kill()
        .times(1)
        .with(eq(514), eq(9))
        .returning(|_, _| Ok(514));
    os.expect_waitpid()
        .times(1)
        .with(eq(514), eq(0))
        .returning(|_, _| Ok(Some(514)));
    let child = InnerRusageChild::new(child, Duration::from_secs(3), os, sig);
    drop(child);
}

#[test]
fn retry() {
    let mut count = 0;
    let res = cvt_r(move || {
        if count == 0 {
            count += 1;
            Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Interrupted",
            ))
        } else {
            Ok(114)
        }
    })
    .expect("Interrupted should succeed");
    assert_eq!(res, 114);
}

#[tokio::test]
async fn wait_twice() {
    let os = MockOs::new();
    let child = get_child(514);
    let sig = get_signal();
    let mut child = InnerRusageChild::new(child, Duration::from_secs(3), os, sig);
    child.result = Some(RunResult {
        rusage: get_rusage(114),
        exit_status: ExitStatus::from_raw(0),
    });
    let res = child.wait().await.expect("Wait twice should succeed");
    assert_eq!(res.rusage.ru_maxrss, 114);
    assert!(res.exit_status.success());
    let res = child
        .try_wait()
        .expect("Wait twice should succeed")
        .expect("Wait twice should succeed");
    assert_eq!(res.rusage.ru_maxrss, 114);
    assert!(res.exit_status.success());
}

#[tokio::test]
async fn interrupted_when_get_signal() {
    let mut os = MockOs::new();
    let child = get_child(114);
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(None))
        .with(eq(114), eq(libc::WNOHANG));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| {
            Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Interrupted",
            ))
        })
        .with(eq(114), eq(libc::WNOHANG));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(Some((0, get_rusage(514)))))
        .with(eq(114), eq(libc::WNOHANG));
    let sig = get_signal();
    let mut child = InnerRusageChild::new(child, Duration::from_secs(3), os, sig);
    let res = child.wait().await.expect("A valid child should succeed");
    assert_eq!(res.rusage.ru_maxrss, 514);
    assert!(res.exit_status.success());
}

#[tokio::test]
async fn doesnt_stop_when_get_signal() {
    let mut os = MockOs::new();
    let child = get_child(114);
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(None))
        .with(eq(114), eq(libc::WNOHANG));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(None))
        .with(eq(114), eq(libc::WNOHANG));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(Some((0, get_rusage(514)))))
        .with(eq(114), eq(libc::WNOHANG));
    let sig = get_signal();
    let mut child = InnerRusageChild::new(child, Duration::from_secs(3), os, sig);
    let res = child.wait().await.expect("A valid child should succeed");
    assert_eq!(res.rusage.ru_maxrss, 514);
    assert!(res.exit_status.success());
}

#[tokio::test]
async fn wait_failed_when_get_signal() {
    let mut os = MockOs::new();
    let child = get_child(114);
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(None))
        .with(eq(114), eq(libc::WNOHANG));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Err(std::io::Error::other("other")))
        .with(eq(114), eq(libc::WNOHANG));
    os.expect_kill().returning(|_, _| Ok(514));
    os.expect_waitpid().returning(|_, _| Ok(Some(514)));
    let sig = get_signal();
    let mut child = InnerRusageChild::new(child, Duration::from_secs(3), os, sig);
    child.wait().await.expect_err("A invalid child should fail");
}

#[tokio::test(start_paused = true)]
async fn kill_failed_when_timeout() {
    let mut os = MockOs::new();
    let child = get_child(114);
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(None))
        .with(eq(114), eq(libc::WNOHANG));
    os.expect_kill()
        .times(1)
        .returning(|_, _| Err(std::io::Error::other("other")));
    os.expect_kill().returning(|_, _| Ok(514));
    os.expect_waitpid().returning(|_, _| Ok(Some(514)));
    let sig = get_no_signal();
    let mut child = InnerRusageChild::new(child, Duration::from_secs(3), os, sig);
    child.wait().await.expect_err("A invalid child should fail");
}

#[tokio::test(start_paused = true)]
async fn wait_failed_when_timeout() {
    let mut os = MockOs::new();
    let child = get_child(114);
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(None))
        .with(eq(114), eq(libc::WNOHANG));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Err(std::io::Error::other("other")));
    os.expect_kill().returning(|_, _| Ok(514));
    os.expect_waitpid().returning(|_, _| Ok(Some(514)));
    let sig = get_no_signal();
    let mut child = InnerRusageChild::new(child, Duration::from_secs(3), os, sig);
    child.wait().await.expect_err("A invalid child should fail");
}
