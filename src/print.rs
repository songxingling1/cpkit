use anstream::eprintln;
use anstyle::Style;

/// 打印进度。
///
/// 输出格式形如
/// ```text
///      Testing xxx
///      Testing yyy
///     Finished zzz
/// ```
///
/// # 参数
/// - `action`: 日志最开始的一个字符串。例如以上例子的 `Testing` 和 `Finished`。
///   打印出来的是绿色粗体字，并且右对齐。
/// - `description`：在 `action` 之后输出的字符串，例如以上例子的 `xxx` `yyy` `zzz`。
pub fn print_progress(action: &str, description: &str) {
    let green = Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)))
        .bold();
    eprintln!("{green}{action:>12}{green:#} {description}");
}

/// 打印错误。
///
/// 输出格式形如
/// ```text
/// error: xxx.
/// ```
/// 其中的 `error` 是红色粗体字。
///
/// # 参数
/// - `message`：报错文本，例如以上示例中的 `xxx.`。
pub fn print_error(message: &str) {
    let red = Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red)))
        .bold();
    eprintln!("{red}error{red:#}: {message}");
}

/// 打印警告。
///
/// 输出格式形如
/// ```text
/// warning: xxx.
/// ```
/// 其中的 `warning` 是黄色粗体字。
///
/// # 参数
/// - `message`：警告文本，例如以上示例中的 `xxx.`。
pub fn print_warn(message: &str) {
    let red = Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)))
        .bold();
    eprintln!("{red}warning{red:#}: {message}");
}
