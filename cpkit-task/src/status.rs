#[cfg(unix)]
mod unix {
    use std::{fmt::Display, num::NonZeroI32};

    use rustix::process::Signal;

    #[derive(Debug)]
    pub enum PrintableStatus {
        Known(Signal),
        Unknown(NonZeroI32),
        ExitCode(i32),
    }

    impl Display for PrintableStatus {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Unknown(val) => write!(f, "Unknown Signal({val})"),
                Self::ExitCode(code) => write!(f, "{code}"),
                Self::Known(sig) => f.write_str(match *sig {
                    Signal::HUP => "Hangup",
                    Signal::INT => "Interrupt",
                    Signal::QUIT => "Quit",
                    Signal::ILL => "Illegal instruction",
                    Signal::TRAP => "Trace/breakpoint trap",
                    Signal::ABORT => "Aborted",
                    Signal::FPE => "Floating point exception",
                    Signal::KILL => "Killed",
                    Signal::BUS => "Bus error",
                    Signal::SYS => "Bad system call",
                    Signal::SEGV => "Segmentation fault",
                    Signal::PIPE => "Broken pipe",
                    Signal::ALARM => "Alarm clock",
                    Signal::TERM => "Terminated",
                    Signal::URG => "Urgent I/O condition",
                    Signal::STOP => "Stopped (signal)",
                    Signal::TSTP => "Stopped",
                    Signal::CONT => "Continued",
                    Signal::CHILD => "Child exited",
                    Signal::TTIN => "Stopped (tty input)",
                    Signal::TTOU => "Stopped (tty output)",
                    Signal::XCPU => "CPU time limit exceeded",
                    Signal::XFSZ => "File size limit exceeded",
                    Signal::VTALARM => "Virtual timer expired",
                    Signal::PROF => "Profiling timer expired",
                    Signal::USR1 => "User defined signal 1",
                    Signal::USR2 => "User defined signal 2",
                    Signal::WINCH => "Window changed",
                    _ => unreachable!(
                        "A valid signal must belong to one of the predefined signals in rustix."
                    ),
                }),
            }
        }
    }
}

#[cfg(unix)]
pub use unix::PrintableStatus;

#[cfg(windows)]
mod windows {
    use std::fmt::Display;

    pub struct PrintableStatus(pub u32);
    impl Display for PrintableStatus {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if self.0 & 0x80000000 != 0 {
                write!(f, "{:#x}", self.0)
            } else {
                write!(f, "{}", self.0)
            }
        }
    }
}

#[cfg(windows)]
pub use windows::PrintableStatus;
