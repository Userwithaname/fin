use crate::font_page::FontPage;
use crate::Action;
use crate::Args;
use crate::InstalledFonts;
use crate::Installer;

use std::collections::btree_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::{fmt, fs};

#[derive(Debug, PartialEq)]
pub enum FontParseError {
    Generic(String),
    InvalidName,
}

impl fmt::Display for FontParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                Self::Generic(e) => e.to_string(),
                Self::InvalidName => "The font name is invalid".to_string(),
            }
        )
    }
}

pub struct Font {
    pub name: String,
    pub installer: Option<Installer>,
    pub override_version: Option<String>,
}

impl fmt::Display for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            self.name,
            match &self.override_version {
                Some(ver) => format!(" {ver}"),
                None => String::new(),
            },
        )
    }
}

impl Font {
    pub fn parse(
        args: &Args,
        name: &str,
        needs_installer: bool,
        cached_pages: &mut HashMap<u64, FontPage>,
    ) -> Result<Self, FontParseError> {
        if name.is_empty() {
            return Err(FontParseError::InvalidName);
        }

        // TODO: The `font:version` format no longer works
        //       due to how the wildcard font matching is implemented
        let mut s = name.split(':');
        if let (Some(name), version, None) = (s.next(), s.next(), s.next()) {
            return Ok(Self {
                name: name.to_string(),
                installer: match needs_installer {
                    true => match Installer::parse(args, name, version, cached_pages) {
                        Ok(installer) => Some(installer),
                        Err(e) => {
                            eprintln!("{e}");
                            None
                        }
                    },
                    false => None,
                },
                override_version: version.map(|v| v.to_string()),
            });
        }
        eprintln!("Invalid format: '{name}'");
        Err(FontParseError::InvalidName)
    }

    pub fn remove(&self, installed_fonts: &mut InstalledFonts) -> Result<(), String> {
        print!("Removing {} ... ", self.name);
        // todo!("Remove the font from disk");
        match installed_fonts.installed.entry(self.name.clone()) {
            Occupied(val) => {
                // TODO: Validate the path before using it to prevent potential data loss
                match fs::remove_dir_all(val.get().dir.clone()).map_err(|e| e.to_string()) {
                    Ok(_) => println!("\x1b[92mDone\x1b[0m"),
                    Err(e) => {
                        println!("\x1b[91m{e}\x1b[0m");
                    }
                }
                installed_fonts.remove_entry(&self.name)?;
            }
            Vacant(_) => {
                println!();
                return Err("Font does not have an installed path associated".to_string());
            }
        };

        Ok(())
    }

    pub fn get_actionable_fonts(
        args: &Args,
        filter: &[String],
        installed_fonts: &mut InstalledFonts,
    ) -> Result<Vec<Font>, FontParseError> {
        let needs_installer;
        let actionable_fonts: Vec<String> = match args.action {
            Action::Install => {
                if filter.is_empty() {
                    println!("No fonts were specified.");
                    return Ok(vec![]);
                }

                let fonts = Installer::find_installers(filter)
                    .map_err(|e| FontParseError::Generic(e.to_string()))?;

                if fonts.is_empty() {
                    println!("Pattern did not match any installers.");
                    return Ok(vec![]);
                }

                needs_installer = true;
                fonts
            }
            Action::Update => {
                let match_all = &["*".to_string()];
                let fonts = Installer::find_installed(
                    match filter.is_empty() {
                        true => match_all,
                        false => filter,
                    },
                    installed_fonts,
                )
                .map_err(|e| FontParseError::Generic(e.to_string()))?;

                if fonts.is_empty() {
                    println!("Pattern did not match any installed fonts.");
                    return Ok(vec![]);
                }

                needs_installer = true;
                fonts
            }
            Action::Remove => {
                if filter.is_empty() {
                    println!("No fonts were specified.");
                    return Ok(vec![]);
                }

                let fonts = Installer::find_installed(filter, installed_fonts)
                    .map_err(|e| FontParseError::Generic(e.to_string()))?;

                if fonts.is_empty() {
                    println!("Pattern did not match any installed fonts.");
                    return Ok(vec![]);
                }

                needs_installer = false;
                fonts
            }
            Action::Help => {
                return Ok(vec![]);
            }
        };

        let mut cached_pages = HashMap::<u64, FontPage>::new();
        fs::create_dir_all(cache_dir!()).map_err(|e| FontParseError::Generic(e.to_string()))?;

        actionable_fonts
            .iter()
            .map(|font| Font::parse(args, font, needs_installer, &mut cached_pages))
            .filter(|font| match args.action {
                Action::Update | Action::Install if !args.options.reinstall => font
                    .as_ref()
                    .unwrap()
                    .installer
                    .as_ref()
                    .is_some_and(|installer| installer.has_updates(installed_fonts)),
                _ => true,
            })
            .collect()
    }
}
