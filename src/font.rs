use crate::font_page::FontPage;
use crate::Action;
use crate::Args;
use crate::InstalledFonts;
use crate::Installer;

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::{fmt, fs};

#[derive(Debug, Eq, PartialEq)]
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
            }
        )
    }
}

impl Font {
    pub fn parse(
        args: Arc<Args>,
        name: &str,
        needs_installer: bool,
        cached_pages: Arc<Mutex<HashMap<String, FontPage>>>,
    ) -> Result<Self, FontParseError> {
        if name.is_empty() {
            return Err(FontParseError::InvalidName);
        }

        let mut s = name.split(':');
        if let (Some(name), version, None) = (s.next(), s.next(), s.next()) {
            return Ok(Self {
                name: name.to_string(),
                installer: if needs_installer {
                    match Installer::parse(&args, installers_dir!(), name, version, cached_pages) {
                        Ok(installer) => Some(installer),
                        Err(e) => {
                            eprintln!("{e}");
                            return Err(FontParseError::Generic(e));
                        }
                    }
                } else {
                    None
                },
                override_version: version.map(ToString::to_string),
            });
        }
        eprintln!("Invalid format: '{name}'");
        Err(FontParseError::InvalidName)
    }

    pub fn get_actionable_fonts(
        args: &Arc<Args>,
        filters: &[String],
        installed_fonts: &Arc<Mutex<InstalledFonts>>,
    ) -> Result<Vec<Font>, FontParseError> {
        let needs_installer;
        let actionable_fonts: Vec<String> = match args.action {
            Action::Install => {
                if filters.is_empty() {
                    println!("No fonts were specified.");
                    return Ok(vec![]);
                }

                let fonts =
                    Installer::filter_installers(filters).map_err(FontParseError::Generic)?;

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

                let fonts = Installer::filter_installed(filters, installed_fonts);

                if fonts.is_empty() {
                    return Ok(vec![]);
                }

                needs_installer = true;
                fonts
            }
            Action::Update => {
                let match_all = &["*".to_string()];
                let fonts = Installer::filter_installed(
                    match filters.is_empty() {
                        true => match_all,
                        false => filters,
                    },
                    installed_fonts,
                );

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

                let fonts = Installer::filter_installed(filters, installed_fonts);

                if fonts.is_empty() {
                    return Ok(vec![]);
                }

                needs_installer = false;
                fonts
            }
            Action::List => {
                let usage = "\
Usage:
    fin list [item]

Items:
    installed             List installed fonts
    available             List available installers
    all                   List all fonts and installers
    help                  Show this help message
";

                let item = match filters.is_empty() {
                    false => filters[0].as_str(),
                    true => "all",
                };
                let fonts = match item {
                    "installed" => Installer::filter_installed(&["*".to_string()], installed_fonts),
                    "available" | "installers" => Installer::filter_installers(&["*".to_string()])
                        .map_err(FontParseError::Generic)?,
                    "all" => {
                        let match_all = &["*".to_string()];
                        let mut fonts = BTreeSet::new();
                        for installer in Installer::filter_installers(match_all)
                            .map_err(FontParseError::Generic)?
                        {
                            fonts.insert(installer);
                        }
                        for installed in Installer::filter_installed(match_all, installed_fonts) {
                            fonts.replace(installed);
                        }
                        fonts.iter().map(ToString::to_string).collect()
                    }
                    "help" => {
                        print!("{usage}");
                        return Ok(vec![]);
                    }
                    item => {
                        println!("{usage}\nCannot list: '{item}'");
                        return Ok(vec![]);
                    }
                };

                needs_installer = false;
                fonts
            }
            Action::Clean | Action::Config | Action::Version | Action::Help => {
                return Ok(vec![]);
            }
        };

        let cached_pages = Arc::new(Mutex::new(HashMap::<String, FontPage>::new()));
        fs::create_dir_all(page_cache_dir!())
            .map_err(|e| FontParseError::Generic(e.to_string()))?;

        let mut handles = Vec::new();
        for font in actionable_fonts {
            let args = Arc::clone(args);
            let cached_pages = Arc::clone(&cached_pages);
            handles.push(thread::spawn(move || {
                Font::parse(args, &font, needs_installer, cached_pages)
            }));
        }

        let mut actionable_fonts = Vec::new();
        for handle in handles {
            let font = handle.join().unwrap();
            if font.is_err() {
                continue;
            }
            let installer = font.as_ref().unwrap().installer.as_ref();
            if match args.action {
                Action::Update | Action::Install if !args.options.reinstall => {
                    installer.unwrap().has_updates(installed_fonts)
                }
                _ => !needs_installer || font.as_ref().unwrap().installer.is_some(),
            } {
                match font {
                    Ok(font) => actionable_fonts.push(font),
                    Err(e) => eprintln!("Error parsing font: {e}"),
                }
            }
        }

        Ok(actionable_fonts)
    }

    #[must_use]
    pub fn has_installer(name: &str) -> bool {
        Path::new(&(installers_dir!() + name)).exists()
    }
}
