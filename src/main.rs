use cpkit::print::print_error;

#[tokio::main]
async fn main() {
    if let Err(e) = cpkit::app().await {
        print_error(&format!("{e:?}"));
        std::process::exit(1);
    }
}
