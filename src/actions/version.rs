pub struct VersionAction;

pub const VERSION: &str = "0.1.0";

impl VersionAction {
    pub fn show_help() -> String {
        let help = "\
Action:
    Show the current version number

Usage:
    fin version
";
        print!("{help}");
        help.to_string()
    }

    pub fn run() {
        println!("{VERSION}");
    }
}
