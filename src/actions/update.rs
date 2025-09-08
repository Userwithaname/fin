use std::sync::{Arc, Mutex};

use crate::args::Args;
use crate::font::Font;
use crate::installed::InstalledFonts;
use crate::user_prompt;

pub struct UpdateAction;

use crate::actions::install::install_fonts;

impl UpdateAction {
    pub fn show_help() -> String {
        let help = "\
Usages:
    fin update
    fin update [font(s)]
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
        println!("Updating: ");
        Args::list_fonts_green(&fonts);
        if !user_prompt("Proceed?", args) {
            return Ok(());
        }

        install_fonts(args, fonts, installed_fonts)
    }
}
