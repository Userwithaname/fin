use std::fs;
use std::sync::{Arc, Mutex};

use crate::Args;
use crate::{font::Font, installed::InstalledFonts};

pub struct ListAction;

impl ListAction {
    pub fn show_help() -> String {
        let help = "\
Action:
    List installed or available fonts
    Lists all when unspecified

Usage:
    fin list
    fin list [item]

Items:
    installed             List installed fonts
    available             List available installers
    all                   List all fonts
";
        print!("{help}");
        help.to_string()
    }

    pub fn run(args: &Args, fonts: &[Font], installed_fonts: Arc<Mutex<InstalledFonts>>) {
        fonts.iter().for_each(|font| {
            if let Some(installed) = installed_fonts.lock().unwrap().installed.get(&font.name) {
                if Font::has_installer(&font.name) {
                    match fs::exists(installed.get_dir()).unwrap_or_default() {
                        true => println_green!("{font}"),
                        false => println_orange!("{font} (missing directory)"),
                    }
                    if args.options.verbose || args.config.verbose_list {
                        println!(" ↪ {}", installed.dir);
                    }
                } else {
                    println_orange!("{font} (missing installer)");
                    if args.options.verbose {
                        println!(" ↪ {}", installed.dir);
                    }
                }
            } else {
                println!("{font}");
            }
        });
    }
}
