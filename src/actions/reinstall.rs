use std::sync::{Arc, Mutex};

use crate::args::Args;
use crate::font::Font;
use crate::installed::InstalledFonts;
use crate::user_prompt;

pub struct ReinstallAction;

use crate::actions::install::install_fonts;

impl ReinstallAction {
    pub fn show_help() -> String {
        let help = "\
Usage:
    fin reinstall [font(s)]
"
        .to_string();

        print!("{help}");

        help
    }

    pub fn run(
        args: &Args,
        fonts: &mut Box<[Font]>,
        installed_fonts: &Arc<Mutex<InstalledFonts>>,
    ) -> Result<(), String> {
        println!("Reinstalling: ");
        Args::list_fonts_green(&fonts);
        if !user_prompt("Proceed?", args) {
            return Ok(());
        }

        install_fonts(args, fonts, installed_fonts)
    }
}
