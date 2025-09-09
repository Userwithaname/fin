use std::sync::{Arc, Mutex};

use crate::args::Args;
use crate::font::Font;
use crate::installed::InstalledFonts;
use crate::user_prompt;

pub struct RemoveAction;

impl RemoveAction {
    pub fn show_help() -> String {
        let help = "\
Usage:
    fin remove [font(s)]
"
        .to_string();

        print!("{help}");

        help
    }

    pub fn run(
        args: &Args,
        fonts: &[Font],
        installed_fonts: &Arc<Mutex<InstalledFonts>>,
    ) -> Result<(), String> {
        println!("Removing: ");
        Args::list_fonts_red(fonts);

        if !user_prompt("Proceed?", args) {
            return Ok(());
        }

        remove_fonts(args, fonts, installed_fonts)
    }
}

fn remove_fonts(
    args: &Args,
    fonts: &[Font],
    installed_fonts: &Arc<Mutex<InstalledFonts>>,
) -> Result<(), String> {
    fonts.iter().try_for_each(|font| {
        installed_fonts
            .lock()
            .unwrap()
            .uninstall(args, &font.name)
            .map(|_| ())
    })?;
    Ok(())
}
