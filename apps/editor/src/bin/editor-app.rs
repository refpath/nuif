#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    if let Err(error) = nuif_editor::gui::run() {
        eprintln!(
            "{{\"error\":{}}}",
            serde_json::to_string(&error).expect("editor launch errors serialize")
        );
        std::process::exit(1);
    }
}
