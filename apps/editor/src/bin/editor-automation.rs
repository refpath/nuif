fn main() {
    if let Err(error) = nuif_editor::gui::automation::run() {
        eprintln!(
            "{{\"error\":{}}}",
            serde_json::to_string(&error).expect("automation errors serialize")
        );
        std::process::exit(1);
    }
}
