use crate::args::*;
use crate::config::Config;
use crate::font::Font;
use crate::installed::InstalledFonts;
use crate::installer::Installer;

use std::io::{self, Write};

pub mod args;
pub mod config;
pub mod font_page;
pub mod installed;
pub mod paths;
pub mod wildcards;

mod font;
mod installer;

pub fn run(args: &Args, installed_fonts: &mut InstalledFonts) -> Result<(), String> {
    match args.action {
        Action::Install => 'install: {
            if args.fonts.is_empty() {
                println!("Nothing new to install.");
                return Ok(());
            }

            println!("Installing: ");
            args.fonts
                .iter()
                .for_each(|i| println!("   \x1b[92m{i}\x1b[0m"));
            println!();

            // TODO: Inform the user of the total download size
            if !user_prompt("Proceed?") {
                break 'install;
            }
            args.fonts.iter().try_for_each(|font| {
                if let Some(installer) = &font.installer {
                    // TODO: Handle the error in a way that doesn't halt the program(?)
                    installer
                        .download_font()?
                        .install_font(args, installed_fonts)
                } else {
                    Err(format!("Installer for '{font}' has not been loaded"))
                }
            })?;
        }
        Action::Update => 'update: {
            if args.fonts.is_empty() {
                println!("Nothing to update.");
                return Ok(());
            }

            println!("Updating: ");
            args.fonts
                .iter()
                .for_each(|i| println!("   \x1b[92m{i}\x1b[0m"));
            println!();

            if !user_prompt("Proceed?") {
                break 'update;
            }
            args.fonts.iter().try_for_each(|font| {
                if let Some(installer) = &font.installer {
                    installer
                        .download_font()?
                        .install_font(args, installed_fonts)
                } else {
                    Err(format!("Installer for '{font}' has not been loaded"))
                }
            })?;
        }
        Action::Remove => 'remove: {
            if args.fonts.is_empty() {
                println!("Nothing to remove.");
                return Ok(());
            }

            println!("Removing: ");
            args.fonts
                .iter()
                .for_each(|i| println!("   \x1b[91m{i}\x1b[0m"));
            println!();

            if !user_prompt("Proceed?") {
                break 'remove;
            }
            println!();
            args.fonts
                .iter()
                .try_for_each(|font| font.remove(installed_fonts))?
        }
        Action::Help => (),
    };

    installed_fonts.write()?;

    Ok(())
}

pub fn user_prompt(message: &str) -> bool {
    print!("{message} [y/n]: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    match input.to_lowercase().as_str() {
        "y\n" | "yes\n" | "yabadabadoo\n" => true,
        "n\n" | "no\n" | "nope\n" => false,
        _ => user_prompt(message),
    }
}
