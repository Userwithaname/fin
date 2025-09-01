use crate::action::Action;
use crate::args::{show_help, Args};
use crate::config::Config;
use crate::font::Font;
use crate::installed::InstalledFonts;
use crate::installer::Installer;

use std::fs;
use std::io::{self, Write};

pub mod action;
pub mod args;
pub mod colors;
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
            args.config.panic_if_invalid();

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
            args.config.panic_if_invalid();

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
            args.config.panic_if_invalid();

            if args.fonts.is_empty() {
                println!("No updates found.");
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

            remove_fonts(args, installed_fonts)?;
        }
        Action::List => {
            args.fonts
                .iter()
                .for_each(|font| match installed_fonts.installed.get(&font.name) {
                    Some(installed) => {
                        if Font::has_installer(&font.name) {
                            match args.options.verbose {
                                true => println!("{}: {}", format_green!("{font}"), installed.dir),
                                false => println_green!("{font}"),
                            }
                        } else {
                            match args.options.verbose {
                                true => println!(
                                    "{}: {}",
                                    format_orange!("{font} (missing installer)"),
                                    installed.dir
                                ),
                                false => println_orange!("{font} (missing installer)"),
                            }
                        }
                    }
                    None => {
                        println!("{font}");
                    }
                });
        }
        Action::Clean => {
            fs::remove_dir_all(cache_dir!()).map_err(|e| e.to_string())?;
            println!("Removed the cache directory: {}", cache_dir!());
        }
        Action::Help => (),
    }

    installed_fonts.write()?;

    Ok(())
}

fn install_fonts(args: &Args, installed_fonts: &mut InstalledFonts) -> Result<(), String> {
    let mut errors = Vec::new();
    args.fonts.iter().for_each(|font| {
        if let Some(installer) = &font.installer {
            match installer
                .download_font()
                .map(|installer| installer.install_font(args, installed_fonts))
            {
                Ok(_) => (),
                Err(e) => {
                    println!("Failed to install {}:\n{}", installer.name, red!(&e));
                    errors.push(format!("{font}: {}", red!(&e)));
                }
            }
        } else {
            println!("Failed to install {font}");
            println_red!("Installer for '{font}' has not been loaded");
            errors.push(format!(
                "{}: {}",
                font,
                red!("Installer has not been loaded")
            ));
        }
    });

    if errors.is_empty() {
        Ok(())
    } else {
        println!("\nFailed:");
        errors.iter().for_each(|e| println!("   {e}"));
        Err("One or more fonts failed to install".to_string())
    }
}

fn remove_fonts(args: &Args, installed_fonts: &mut InstalledFonts) -> Result<(), String> {
    args.fonts.iter().try_for_each(|font| {
        println!();
        installed_fonts.uninstall(&font.name, args).map(|_| ())
    })?;
    Ok(())
}

#[inline]
#[must_use]
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
