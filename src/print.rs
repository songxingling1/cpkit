use anstream::eprintln;
use anstyle::Style;

pub fn print_progress(action: &str, description: &str) {
    let green = Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)))
        .bold();
    eprintln!("{green}{action:>12}{green:#} {description}");
}

pub fn print_error(message: &str) {
    let red = Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red)))
        .bold();
    eprintln!("{red}error{red:#}: {message}");
}

pub fn print_warn(message: &str) {
    let red = Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)))
        .bold();
    eprintln!("{red}warning{red:#}: {message}");
}
