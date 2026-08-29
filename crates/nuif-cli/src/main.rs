use std::env;

const COMMANDS: &[&str] = &[
    "version",
    "capabilities",
    "inspect",
    "query",
    "validate",
    "canonicalize",
    "diff",
    "patch",
    "layout",
    "render",
    "snapshot",
    "replay",
    "migrate",
    "import",
    "export",
];

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("version") => println!("nuif 0.0.1"),
        Some("capabilities") => print_capabilities(),
        Some(command) if COMMANDS.contains(&command) => {
            eprintln!(
                "{{\"error\":\"not_implemented\",\"command\":\"{command}\",\"status\":\"prototype\"}}"
            );
            std::process::exit(3);
        }
        Some(command) => {
            eprintln!("nuif: unknown command `{command}`");
            std::process::exit(2);
        }
        None => {
            eprintln!("usage: nuif <{}>", COMMANDS.join("|"));
            std::process::exit(2);
        }
    }
}

fn print_capabilities() {
    let commands = COMMANDS
        .iter()
        .map(|command| format!("\"{command}\""))
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"protocol\":\"0.0.1\",\"status\":\"prototype\",\"commands\":[{commands}]}}"
    );
}
