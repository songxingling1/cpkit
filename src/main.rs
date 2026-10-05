use cpkit::print::print_error;

fn main() {
    if let Err(e) = cpkit::app() {
        print_error(&format!("{e:?}"));
        std::process::exit(1);
    }
}
