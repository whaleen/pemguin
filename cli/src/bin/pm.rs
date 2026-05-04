fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if pemguin::run_cli(&args)? {
        Ok(())
    } else {
        pemguin::start()
    }
}
