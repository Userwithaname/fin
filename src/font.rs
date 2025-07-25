use crate::font_page::FontPage;
use crate::Action;
use crate::Args;
use crate::InstalledFonts;
use crate::Installer;

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
                Self::InvalidName => "The name is invalid".to_string(),
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

    pub fn get_actionable_fonts(
        args: &Args,
        filters: &[String],
        installed_fonts: &mut InstalledFonts,
    ) -> Result<Vec<Font>, FontParseError> {
        let needs_installer;
        let actionable_fonts: Vec<String> = match args.action {
            Action::Install => {
                if filters.is_empty() {
                    println!("No fonts were specified.");
                    return Ok(vec![]);
                }

                let fonts = Installer::find_installers(filters)
                    .map_err(|e| FontParseError::Generic(e.to_string()))?;

                if fonts.is_empty() {
                    return Ok(vec![]);
                }

                needs_installer = true;
                fonts
            }
            Action::Reinstall => {
                if filters.is_empty() {
                    println!("No fonts were specified.");
                    return Ok(vec![]);
                }

                let fonts = Installer::find_installed(filters, installed_fonts)
                    .map_err(|e| FontParseError::Generic(e.to_string()))?;

                if fonts.is_empty() {
                    return Ok(vec![]);
                }

                needs_installer = true;
                fonts
            }
            Action::Update => {
                let match_all = &["*".to_string()];
                let fonts = Installer::find_installed(
                    match filters.is_empty() {
                        true => match_all,
                        false => filters,
                    },
                    installed_fonts,
                )
                .map_err(|e| FontParseError::Generic(e.to_string()))?;

                if fonts.is_empty() {
                    return Ok(vec![]);
                }

                needs_installer = true;
                fonts
            }
            Action::Remove => {
                if filters.is_empty() {
                    println!("No fonts were specified.");
                    return Ok(vec![]);
                }

                let fonts = Installer::find_installed(filters, installed_fonts)
                    .map_err(|e| FontParseError::Generic(e.to_string()))?;

                if fonts.is_empty() {
                    return Ok(vec![]);
                }

                needs_installer = false;
                fonts
            }
            Action::List => {
                if filters.is_empty() {
                    println!("Specify what to list: [installed/available]");
                    return Ok(vec![]);
                }
                let fonts = match filters[0].as_str() {
                    "installed" => Installer::find_installed(&["*".to_string()], installed_fonts)
                        .map_err(FontParseError::Generic)?,
                    "available" | "installers" => Installer::find_installers(&["*".to_string()])
                        .map_err(FontParseError::Generic)?,
                    item => {
                        println!(
                            "Cannot list: '{item}'\nSpecify what to list: [installed/available]"
                        );
                        return Ok(vec![]);
                    }
                };

                needs_installer = false;
                fonts
            }
            Action::Clean => {
                return Ok(vec![]);
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
                _ => !needs_installer || font.as_ref().unwrap().installer.is_some(),
            })
            .collect()
    }
}
