pub struct VersionAction;

pub const VERSION: &str = "0.1.0";

impl VersionAction {
    pub fn show_help() -> String {
        let help = "\
Action:
    Show the current version number and exit

Usage:
    fin version
"
        .to_string();

        print!("{help}");

        help
    }

    pub fn run() {
        println!("{VERSION}");
    }
}
