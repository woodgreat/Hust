fn main() {
    let code = hust_conductor::app::run(std::env::args_os().skip(1));
    std::process::exit(code);
}
