use std::time::Duration;

/// 退出信息
#[derive(Clone, Debug)]
pub struct ExitInformation {
    /// 运行时间占用
    pub time_usage: Duration,
    /// 内存占用（MiB）
    pub memory_usage: f64,
    /// 退出状态
    pub status: std::borrow::Cow<'static, str>,
}
