#[cfg(unix)]
mod unix {
    use std::{fmt::Display, num::NonZeroI32};

    use rustix::process::Signal;

    #[derive(Debug)]
    pub enum PrintableStatus {
        None,
        Known(Signal),
        Unknown(NonZeroI32),
    }

    impl Display for PrintableStatus {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::None => f.write_str("None"),
                Self::Unknown(val) => write!(f, "Unknown Signal({val})"),
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
