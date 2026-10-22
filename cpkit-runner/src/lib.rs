mod reactor;
mod r#impl;

#[cfg(target_os = "linux")]
pub type RunnerReactor = reactor::RunnerReactor<r#impl::linux::Epoll>;
#[cfg(target_os = "macos")]
pub type RunnerReactor = reactor::RunnerReactor<r#impl::macos::KQueue>;
#[cfg(target_os = "windows")]
pub type RunnerReactor = reactor::RunnerReactor<r#impl::windows::ProcessPool>;
