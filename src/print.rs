/// 打印进度。
///
/// 输出格式形如
/// ```text
///      Testing xxx
///      Testing yyy
///     Finished zzz
/// ```
/// 第一个参数是步骤，例如上面的 `Testing`。
/// 后面的参数描述后面的注释，例如上面的 `xxx` `yyy` `zzz`。
#[macro_export]
macro_rules! print_progress {
    ($action:literal, $($arg:tt)*) => {
        let green = ::anstyle::Style::new()
            .fg_color(Some(::anstyle::Color::Ansi(::anstyle::AnsiColor::Green)))
            .bold();
        ::anstream::eprint!("{green}{:>12}{green:#} ",$action);
        ::anstream::eprint!($($arg)*);
        ::anstream::eprint!("\n");
    };
}

/// 打印错误。
///
/// 输出格式形如
/// ```text
/// error: xxx.
/// ```
/// 其中的 `error` 是红色粗体字。
#[macro_export]
macro_rules! print_error {
    ($($arg:tt)*) => {
        let red = ::anstyle::Style::new()
            .fg_color(Some(::anstyle::Color::Ansi(::anstyle::AnsiColor::Red)))
            .bold();
        ::anstream::eprint!("{red}error{red:#}: ");
        ::anstream::eprint!($($arg)*);
        ::anstream::eprint!("\n");
    };
}

/// 打印警告。
///
/// 输出格式形如
/// ```text
/// warning: xxx.
/// ```
/// 其中的 `warning` 是黄色粗体字。
#[macro_export]
macro_rules! print_warn {
    ($($arg:tt)*) => {
        let yellow = ::anstyle::Style::new()
            .fg_color(Some(::anstyle::Color::Ansi(::anstyle::AnsiColor::Yellow)))
            .bold();
        ::anstream::eprint!("{yellow}warning{yellow:#}: ");
        ::anstream::eprint!($($arg)*);
        ::anstream::eprint!("\n");
    };
}
