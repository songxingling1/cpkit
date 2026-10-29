mod imp;
mod status;
mod task;
mod taskset;

cfg_select! {
    target_os = "linux" => {
        pub use taskset::TaskSet;
    }
}

pub use task::{Task, TaskResult};
