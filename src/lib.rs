use crate::action::Action;
use crate::args::*;
use crate::config::Config;
use crate::font::Font;
use crate::installed::InstalledFonts;
use crate::installer::Installer;

use std::fs;
use std::io::{self, Write};

pub mod action;
pub mod args;
pub mod config;
pub mod font_page;
pub mod installed;
pub mod options;
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
            args.list_fonts_green();

            // TODO: Inform the user of the total download size
            if !user_prompt("Proceed?", args) {
                break 'install;
            }

            install_fonts(args, installed_fonts)?;
        }
        Action::Reinstall => 'reinstall: {
            if args.fonts.is_empty() {
                println!("Nothing to reinstall.");
                return Ok(());
            }

            println!("Installing: ");
            args.list_fonts_green();

            if !user_prompt("Proceed?", args) {
                break 'reinstall;
            }

            install_fonts(args, installed_fonts)?;
        }
        Action::Update => 'update: {
            if args.fonts.is_empty() {
                println!("Nothing to update.");
                return Ok(());
            }

            println!("Updating: ");
            args.list_fonts_green();

            if !user_prompt("Proceed?", args) {
                break 'update;
            }

            install_fonts(args, installed_fonts)?;
        }
        Action::Remove => 'remove: {
            if args.fonts.is_empty() {
                println!("Nothing to remove.");
                return Ok(());
            }

            println!("Removing: ");
            args.list_fonts_red();

            if !user_prompt("Proceed?", args) {
                break 'remove;
            }

            println!();
            remove_fonts(args, installed_fonts)?;
        }
        Action::List => {
            args.fonts.iter().for_each(|font| println!("{font}"));
        }
        Action::Clean => {
            fs::remove_dir_all(cache_dir!()).map_err(|e| e.to_string())?;
            println!("Removed the cache directory: {}", cache_dir!());
        }
        Action::Help => (),
    };

    installed_fonts.write()?;

    Ok(())
}

fn install_fonts(args: &Args, installed_fonts: &mut InstalledFonts) -> Result<(), String> {
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
    Ok(())
}

fn remove_fonts(args: &Args, installed_fonts: &mut InstalledFonts) -> Result<(), String> {
    args.fonts
        .iter()
        .try_for_each(|font| font.remove(installed_fonts))?;
    Ok(())
}

pub fn user_prompt(message: &str, args: &Args) -> bool {
    print!("{message} [y/n]: ");

    match args.options.answer {
        Some(false) => {
            println!("no");
            return false;
        }
        Some(true) => {
            println!("yes");
            return true;
        }
        None => {}
    }

    let mut input = String::new();
    let _ = io::stdout().flush();
    io::stdin().read_line(&mut input).unwrap();

    match input.to_lowercase().as_str() {
        "y\n" | "yes\n" | "yabadabadoo\n" => true,
        "n\n" | "no\n" | "nope\n" => false,
        _ => user_prompt(message, args),
    }
}
