mod r#impl;
mod reactor;

cfg_select! {
    target_os = "linux" => {
        pub type RunnerReactor = reactor::RunnerReactor<r#impl::linux::Epoll>;
        pub type RunnerReactorHandle = reactor::RunnerReactorHandle<r#impl::linux::Epoll>;
        pub type Registration = reactor::Registration<r#impl::linux::Epoll>;
    }
    target_os = "windows" => {
        pub type RunnerReactor = reactor::RunnerReactor<r#impl::windows::ProcessPool>;
        pub type RunnerReactorHandle = reactor::RunnerReactorHandle<r#impl::windows::ProcessPool>;
        pub type Registration = reactor::Registration<r#impl::windows::ProcessPool>;
    }
    target_os = "macos" => {
        pub type RunnerReactor = reactor::RunnerReactor<r#impl::macos::KQueue>;
        pub type RunnerReactorHandle = reactor::RunnerReactorHandle<r#impl::macos::KQueue>;
        pub type Registration = reactor::Registration<r#impl::macos::KQueue>;
    }
}
