use crate::rusage::{ExitInformation, InnerRusageChild, MockChildLike, MockOs};
use std::{os::unix::process::ExitStatusExt, process::ExitStatus};

fn get_exit_information(mark: i64) -> ExitInformation {
    let time = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let exit = ExitStatus::from_raw(0);
    let rusage = libc::rusage {
        ru_idrss: 0,
        ru_inblock: 0,
        ru_isrss: 0,
        ru_ixrss: 0,
        ru_majflt: 0,
        ru_maxrss: mark,
        ru_minflt: 0,
        ru_msgrcv: 0,
        ru_msgsnd: 0,
        ru_nivcsw: 0,
        ru_nsignals: 0,
        ru_nswap: 0,
        ru_nvcsw: 0,
        ru_oublock: 0,
        ru_stime: time,
        ru_utime: time,
    };
    ExitInformation {
        exit_status: exit,
        rusage,
    }
}

#[tokio::test]
async fn success() {
    let mut os = MockOs::new();
    os.expect_recv_child().return_const(());
    os.expect_wait4().times(1).returning(|_, _| Ok(None));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(Some(get_exit_information(114))));
    let mut child = MockChildLike::new();
    child.expect_id().return_const(114);
    child.expect_kill().returning(|| Ok(()));
    let mut test = InnerRusageChild {
        os,
        result: None,
        child,
    };
    test.kill().expect("A valid child should success");
    let res = test.wait().await.expect("A valid child should success");
    assert!(res.exit_status.success());
    assert_eq!(res.rusage.ru_maxrss, 114);
}

#[tokio::test]
async fn not_kill_wait() {
    let os = MockOs::new();
    let child = MockChildLike::new();
    let mut test = InnerRusageChild {
        os,
        result: Some(get_exit_information(114)),
        child,
    };
    test.kill().expect("A exist result shouldn't kill");
    test.try_wait().expect("A exist result shouldn't kill");
    test.wait().await.expect("A exist result shouldn't kill");
}

#[tokio::test]
async fn finish_before_signal() {
    let mut os = MockOs::new();
    os.expect_recv_child().times(0);
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(Some(get_exit_information(114))));
    let mut child = MockChildLike::new();
    child.expect_id().return_const(114);
    let mut test = InnerRusageChild {
        os,
        result: None,
        child,
    };
    let res = test.wait().await.expect("A valid child should success");
    assert!(res.exit_status.success());
    assert_eq!(res.rusage.ru_maxrss, 114);
}

#[test]
fn echld_when_try_wait() {
    let mut os = MockOs::new();
    os.expect_recv_child().times(0);
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Err(std::io::Error::new(std::io::ErrorKind::NotFound, "ECHILD")));
    let mut child = MockChildLike::new();
    child.expect_id().return_const(114);
    let mut test = InnerRusageChild {
        os,
        result: None,
        child,
    };
    let err = test.try_wait().expect_err("A ECHILD error should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[tokio::test]
async fn echld_when_wait() {
    let mut os = MockOs::new();
    os.expect_recv_child().return_const(());
    os.expect_wait4().times(1).returning(|_, _| Ok(None));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Err(std::io::Error::new(std::io::ErrorKind::NotFound, "ECHILD")));
    let mut child = MockChildLike::new();
    child.expect_id().return_const(114);
    let mut test = InnerRusageChild {
        os,
        result: None,
        child,
    };
    let err = test.wait().await.expect_err("A ECHILD error should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[tokio::test]
async fn sigchld_twice() {
    let mut os = MockOs::new();
    os.expect_recv_child().return_const(());
    os.expect_wait4().times(2).returning(|_, _| Ok(None));
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Ok(Some(get_exit_information(114))));
    let mut child = MockChildLike::new();
    child.expect_id().return_const(114);
    let mut test = InnerRusageChild {
        os,
        result: None,
        child,
    };
    let res = test.wait().await.expect("A valid child should success");
    assert!(res.exit_status.success());
    assert_eq!(res.rusage.ru_maxrss, 114);
}

#[tokio::test]
async fn echld_when_wait_calling_try_wait() {
    let mut os = MockOs::new();
    os.expect_recv_child().return_const(());
    os.expect_wait4()
        .times(1)
        .returning(|_, _| Err(std::io::Error::new(std::io::ErrorKind::NotFound, "ECHILD")));
    let mut child = MockChildLike::new();
    child.expect_id().return_const(114);
    let mut test = InnerRusageChild {
        os,
        result: None,
        child,
    };
    let err = test.wait().await.expect_err("A ECHILD error should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}
