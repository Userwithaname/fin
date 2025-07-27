use crate::font_page::FontPage;
use crate::installed::{InstalledFont, InstalledFonts};
use crate::wildcards::*;
use crate::Args;

use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::{self, DirEntry};
use std::io::{self, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Deserialize)]
#[serde(default)]
pub struct Installer {
    name: String,
    tag: String,
    url: String,
    archive: String,
    include: Box<[String]>,
    exclude: Box<[String]>,
    #[serde(skip_serializing)]
    installer_name: String,
}

impl Default for Installer {
    fn default() -> Self {
        Self {
            name: String::new(),
            tag: String::from("latest"),
            url: String::new(),
            archive: String::new(),
            include: Box::new([String::from("*")]),
            exclude: Box::new([]),
            installer_name: String::new(),
        }
    }
}

impl Installer {
    pub fn parse(
        args: &Args,
        font_name: &str,
        override_version: Option<&str>,
        cached_pages: &mut HashMap<u64, FontPage>,
    ) -> Result<Self, String> {
        let mut installer: Self = toml::from_str(
            &fs::read_to_string(installer_path!(&font_name)).map_err(|err| {
                eprintln!("Error: Could not read the installer file for '{font_name}'");
                err.to_string()
            })?,
        )
        .map_err(|err| {
            eprintln!("Could not parse the installer for '{font_name}'");
            err.to_string()
        })?;

        installer.installer_name = font_name.to_string();

        if let Some(version) = override_version {
            installer.tag = version.to_string();
        }

        if !match_wildcard(&installer.url, "*://*.*/*") {
            return Err(format!(
                "Installer for '{font_name}' did not specify a valid URL"
            ));
        }

        let reqwest_client = reqwest::blocking::Client::new();
        installer.url = installer.find_direct_link(FontPage::get_font_page(
            &installer.url.replace("$tag", &installer.tag),
            args,
            &reqwest_client,
            cached_pages,
        )?)?;

        if !match_wildcard(&installer.archive, "*.*") {
            return Err(format!(
                "Installer for '{font_name}' did not specify a valid archive"
            ));
        }
        installer.archive = installer.archive.replace("$tag", &installer.tag);

        installer.include = installer
            .include
            .iter()
            .map(|file| file.replace("$tag", &installer.tag))
            .collect();

        installer.exclude = installer
            .exclude
            .iter()
            .map(|file| file.replace("$tag", &installer.tag))
            .collect();

        Ok(installer)
    }

    pub fn find_installers(filters: &[String]) -> Result<Vec<String>, String> {
        let installers_dir = installers_dir_path!();
        if !Path::new(&installers_dir).exists() {
            return Err(format!(
                "Installers directory does not exist: {installers_dir}"
            ));
        }

        let installers: Vec<String> = fs::read_dir(installers_dir)
            .map_err(|e| e.to_string())?
            .filter_map(|installer| {
                installer.ok().and_then(|i| {
                    i.path()
                        .file_name()
                        .and_then(|n| n.to_str().map(String::from))
                })
            })
            .collect();

        let mut matches = HashMap::<String, Vec<String>>::new();
        for filter in filters {
            let mut p_t = filter.split(':');
            let (pattern, tag) = (p_t.next().unwrap(), p_t.next());

            for input in &installers {
                if !match_wildcard(input, pattern) {
                    continue;
                }

                let font = if let Some(tag) = tag {
                    input.to_string() + ":" + tag
                } else {
                    input.to_string()
                };

                match matches.get_mut(filter) {
                    Some(entry) => entry.push(font),
                    None => {
                        let _ = matches.insert(filter.to_string(), vec![font]);
                    }
                };
            }
        }

        let mut installers = HashSet::new();

        filters.iter().for_each(|filter| match matches.get(filter) {
            Some(fonts) => {
                fonts.iter().for_each(|val| {
                    installers.replace(val.to_owned());
                });
            }
            None => eprintln!("No installers: '{filter}'"),
        });

        Ok(installers.iter().map(|i| i.to_string()).collect())
    }

    pub fn find_installed(
        filters: &[String],
        installed_fonts: &mut InstalledFonts,
    ) -> Result<Vec<String>, String> {
        let installed_fonts = installed_fonts.get_names();

        let matches = match_wildcards_multi(&installed_fonts, filters);
        let mut installed = HashSet::new();

        filters.iter().for_each(|filter| match matches.get(filter) {
            Some(fonts) => {
                fonts.iter().for_each(|val| {
                    installed.replace(val.to_owned());
                });
            }
            None => eprintln!("Not installed: '{filter}'"),
        });

        Ok(installed.iter().map(|i| i.to_string()).collect())
    }

    /// Returns a direct link to the font archive
    /// Note: `self.url` is expected to lead to a webpage, which has the direct link
    /// discoverable in plain text within its source.
    fn find_direct_link(&mut self, font_page: FontPage) -> Result<String, String> {
        font_page
            .contents
            .unwrap()
            .split('"')
            .filter_map(|line| {
                wildcard_substring(line, &(String::from("https://*") + &self.archive), b"")
            })
            .next()
            .map_or(
                Err(format!(
                    "Archive download link not found for {} ({})",
                    self.name, self.tag
                )),
                |link| Ok(link.to_string()),
            )
    }

    pub fn download_font(&self) -> Result<&Installer, String> {
        let reqwest_client = reqwest::blocking::Client::new();

        println!("\n{}:", &self.name);

        println!("Awaiting response: {} ...", &self.url);
        let mut remote_data = reqwest_client
            .get(&self.url)
            .header(USER_AGENT, "fin")
            .send()
            .map_err(|e| e.to_string())?;

        println!("Downloading archive...");
        let mut archive_buffer: Vec<u8> = Vec::new();
        remote_data
            .read_to_end(&mut archive_buffer)
            .map_err(|e| e.to_string())?;

        // println!("Reading archive...");
        let reader = std::io::Cursor::new(archive_buffer);
        let mut zip_archive = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;

        println!("Attempting extraction...");
        // TODO: Extract selectively (instead of selectively moving in `install_font()`)
        zip::ZipArchive::extract(
            &mut zip_archive,
            format!("{}/{}/{}/", cache_dir!(), &self.name, &self.tag),
        )
        .map_err(|e| e.to_string())?;
        Ok(self)
    }

    pub fn install_font(
        &self,
        args: &Args,
        installed_fonts: &mut InstalledFonts,
    ) -> Result<(), String> {
        // TODO: If already installed, remove it before installation?
        let temp_dir = format!("{}/{}/{}/", cache_dir!(), &self.name, &self.tag,);

        let dest_dir = installed_fonts
            .uninstall(&self.installer_name, args)?
            .unwrap_or(format!("{}/{}/", args.config.install_dir, &self.name));

        // Move the files specified by the installer into the target directory
        println!("Installing:");

        fs::create_dir_all(&dest_dir).map_err(|err| err.to_string())?;
        let errors = Arc::new(Mutex::new(false));

        let mut files = Vec::new();
        visit_dirs(Path::new(&temp_dir), &mut |file| {
            let partial_path = &file.path().display().to_string().replace(&temp_dir, "");

            // Ignore files which don't satisfy the 'include' and 'exclude' patterns
            if !match_any_wildcard(partial_path, &self.include)
                || match_any_wildcard(partial_path, &self.exclude)
            {
                return;
            }

            let filename = partial_path.split('/').next_back().unwrap();
            files.push(filename.to_owned());
            print!("   {partial_path} ... ");

            // IDEA: Option to preserve directory structure (specified by the installer)
            // if let Err(e) = fs::create_dir_all(
            //     Path::new(&format!("{dest_dir}/{partial_path}"))
            //         .parent()
            //         .unwrap(),
            // ) {
            //     println!("\x1b[91m{e}\x1b[0m");
            //     *errors.lock().unwrap() = true;
            //     return;
            // }

            match fs::rename(
                format!("{temp_dir}/{partial_path}"),
                format!("{dest_dir}/{filename}"),
                // format!("{dest_dir}/{partial_path}"), // <-- to preserve subdirectories
            ) {
                Ok(_) => println!("\x1b[92mDone\x1b[0m"),
                Err(e) => {
                    println!("\x1b[91m{e}\x1b[0m");
                    *errors.lock().unwrap() = true;
                }
            };
        })
        .map_err(|e| e.to_string())?;

        match *errors.lock().unwrap() {
            false => {
                println!("Successfully installed {}", self.name);
                installed_fonts.update_entry(
                    &self.installer_name,
                    InstalledFont {
                        url: self.url.clone(),
                        dir: dest_dir,
                        files,
                    },
                );
            }
            true => println!("Errors were encountered while installing {}", self.name),
        }

        Ok(())
    }

    pub fn has_updates(&self, installed_fonts: &mut InstalledFonts) -> bool {
        self.url
            != match installed_fonts.installed.get(&self.installer_name) {
                Some(installed) => installed.url.clone(),
                None => String::new(),
            }
    }
}

// https://doc.rust-lang.org/nightly/std/fs/fn.read_dir.html#examples
fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(&DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}
