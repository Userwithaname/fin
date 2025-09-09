use std::sync::{Arc, Mutex};

use crate::args::Args;
use crate::font::Font;
use crate::installed::InstalledFonts;
use crate::installer::Installer;
use crate::user_prompt;

pub struct InstallAction;

impl InstallAction {
    pub fn show_help() -> String {
        let help = "\
Action:
    Install the specified fonts

Usage:
    fin install [font]
    fin install [font]:[tag]
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
        println!("Installing: ");
        Args::list_fonts_green(fonts);

        if !user_prompt("Proceed?", args) {
            return Ok(());
        }

        install_fonts(args, fonts, installed_fonts)
    }
}

// IDEA: Parallel downloads, only install after all downloads are done
pub fn install_fonts(
    args: &Args,
    fonts: &mut Box<[Font]>,
    installed_fonts: &Arc<Mutex<InstalledFonts>>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    fonts.iter_mut().for_each(|font| {
        if let Some(installer) = &mut font.installer {
            match download_and_install(args, installer, installed_fonts) {
                Ok(()) => (),
                Err(e) => {
                    match args.options.verbose || args.config.verbose_files {
                        true => println!("Failed to install {}:\n{}", installer.name, red!(&e)),
                        false => println!("\nFailed to install {}:\n{}", installer.name, red!(&e)),
                    }
                    errors.push(format!("{font}: {}", red!(&e)));
                }
            }
        } else {
            match args.options.verbose || args.config.verbose_files {
                true => println!("Failed to install {font}"),
                false => println!("\nFailed to install {font}"),
            }
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
        println!();
        errors.iter().for_each(|e| println!("{e}"));
        Err("One or more fonts failed to install".to_string())
    }
}

fn download_and_install(
    args: &Args,
    installer: &mut Installer,
    installed_fonts: &Arc<Mutex<InstalledFonts>>,
) -> Result<(), String> {
    match args.options.verbose || args.config.verbose_urls {
        true => println!("\n{} ({}): ", installer.name, installer.url),
        false => println!("\n{}:", installer.name),
    }
    installer
        .download_font()?
        .prepare_install(args)?
        .finalize_install(args, installed_fonts)
}
