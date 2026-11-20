fn main() {
    let result = winedroid::run_cli();
    winedroid::report_missing_primitives();

    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
