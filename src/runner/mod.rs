mod model;
mod unix;
mod wait4;

pub use model::ExitInformation;
pub use wait4::RunnerChild;

pub trait RunnerExt {
    fn spawn_with_runner(&mut self) -> std::io::Result<RunnerChild>;
}

impl RunnerExt for std::process::Command {
    fn spawn_with_runner(&mut self) -> std::io::Result<RunnerChild> {
        RunnerChild::new(self.spawn()?)
    }
}
