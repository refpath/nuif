use std::env;

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("version") => println!("nuif 0.0.1"),
        Some("capabilities") => {
            println!("{{\"protocol\":\"0.0.1\",\"commands\":[\"version\",\"capabilities\"]}}")
        }
        Some(command) => {
            eprintln!("nuif: unknown command `{command}`");
            std::process::exit(2);
        }
        None => {
            eprintln!("usage: nuif <version|capabilities>");
            std::process::exit(2);
        }
    }
}
