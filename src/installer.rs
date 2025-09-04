use crate::font_page::FontPage;
use crate::installed::{InstalledFont, InstalledFonts};
use crate::wildcards::*;
use crate::Args;

use std::collections::{BTreeSet, HashMap};
use std::fs::{self, DirEntry};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use reqwest::header::USER_AGENT;

use flate2::read::GzDecoder;
use tar::Archive;

use serde::Deserialize;

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Installer {
    pub name: String,
    tag: String,
    url: String,
    archive: String,
    include: Box<[String]>,
    exclude: Box<[String]>,
    keep_folders: bool,

    #[serde(skip_serializing)]
    installer_name: String,
    #[serde(skip_serializing)]
    download_buffer: Option<Vec<u8>>,
}

impl Default for Installer {
    fn default() -> Self {
        Self {
            name: String::new(),
            tag: String::new(),
            url: String::new(),
            archive: String::new(),
            include: [String::from("*")].into(),
            exclude: [].into(),
            keep_folders: false,
            installer_name: String::new(),
            download_buffer: None,
        }
    }
}

impl Installer {
    pub fn parse(
        args: Arc<Args>,
        font_name: &str,
        override_version: Option<&str>,
        cached_pages: Arc<Mutex<HashMap<u64, FontPage>>>,
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

    /// Returns the installer names of all available installers matched
    /// by any of the provided filter patterns
    pub fn filter_installers(filters: &[String]) -> Result<Vec<String>, String> {
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

                let font = match tag {
                    Some(tag) => input.to_string() + ":" + tag,
                    None => input.to_string(),
                };

                match matches.get_mut(filter) {
                    Some(entry) => entry.push(font),
                    None => {
                        let _ = matches.insert(filter.to_string(), vec![font]);
                    }
                }
            }
        }

        let mut installers = BTreeSet::new();

        filters.iter().for_each(|filter| match matches.get(filter) {
            Some(fonts) => {
                for font in fonts {
                    installers.replace(font.to_owned());
                }
            }
            None => println!("No installers: '{filter}'"),
        });

        Ok(installers.iter().map(ToString::to_string).collect())
    }

    /// Returns the installer names of all installed fonts matched by
    /// any of the provided filter patterns
    #[must_use]
    pub fn filter_installed(filters: &[String], installed_fonts: &InstalledFonts) -> Vec<String> {
        let installed_fonts = installed_fonts.get_names();

        let matches = match_wildcards_multi(&installed_fonts, filters);
        let mut installed = BTreeSet::new();

        filters.iter().for_each(|filter| match matches.get(filter) {
            Some(fonts) => {
                for font in fonts {
                    installed.replace(font.to_owned());
                }
            }
            None => println!("Not installed: '{filter}'"),
        });

        installed.iter().map(ToString::to_string).collect()
    }

    /// Returns a direct link to the font archive
    /// Note: `self.url` is expected to lead to a webpage, which has the direct link
    /// discoverable in plain text within its source.
    fn find_direct_link(&self, font_page: FontPage) -> Result<String, String> {
        font_page
            .contents
            .unwrap()
            .split('"')
            .find_map(|line| {
                wildcard_substring(line, &(String::from("https://*") + &self.archive), b"")
            })
            .map_or_else(
                || {
                    Err(format!(
                        "Archive download link not found for {} ({})",
                        self.name, self.tag
                    ))
                },
                |link| Ok(link.to_string()),
            )
    }

    pub fn download_font(&mut self) -> Result<&mut Self, String> {
        let reqwest_client = reqwest::blocking::Client::new();

        println!("\n{}:", &self.name);

        print!("Awaiting response: {} ... ", &self.url);
        let _ = io::stdout().flush();

        let mut remote_data = reqwest_client
            .get(&self.url)
            .header(USER_AGENT, "fin")
            .send()
            .map_err(|e| {
                println_red!("Error");
                e.to_string()
            })?;
        println_green!("OK");

        print!("Downloading archive... ");
        let _ = io::stdout().flush();

        // TODO: Show download progress
        let mut archive_buffer: Vec<u8> = Vec::new();
        remote_data.read_to_end(&mut archive_buffer).map_err(|e| {
            println_red!("Failed");
            e.to_string()
        })?;
        self.download_buffer = Some(archive_buffer);
        println_green!("Done");

        Ok(self)
    }

    pub fn extract_archive(&mut self) -> Result<&Self, String> {
        let data = self.download_buffer.take();
        if data.is_none() {
            return Err(format!(
                "{}: {}",
                self.installer_name,
                red!("Attempted extraction with no downloaded data"),
            ));
        }

        let reader = std::io::Cursor::new(data.unwrap());
        let extract_to = format!("{}/{}/{}/", cache_dir!(), &self.name, &self.tag);

        match self.archive.split('.').next_back() {
            Some("zip") => self.extract_zip(reader, &extract_to)?,
            Some("gz") => self.extract_tar_gz(reader, &extract_to)?,
            Some("xz") => self.extract_tar_xz(reader, &extract_to)?,
            Some(_) => return Err(format!("Unsupported archive extension: {}", self.archive)),
            None => {
                return Err(format!(
                    "Archive requires a file extension: {}",
                    self.archive
                ))
            }
        }

        Ok(self)
    }

    fn extract_zip(
        &self,
        reader: std::io::Cursor<Vec<u8>>,
        extract_to: &str,
    ) -> Result<(), String> {
        print!("Attempting extraction... ");
        let _ = io::stdout().flush();

        let mut zip_archive = zip::ZipArchive::new(reader).map_err(|e| {
            println_red!("Failed");
            e.to_string()
        })?;

        // TODO: Extract selectively (instead of selectively moving in `install_font()`)
        zip::ZipArchive::extract(&mut zip_archive, extract_to).map_err(|e| {
            println_red!("Failed");
            e.to_string()
        })?;
        println_green!("Done");

        Ok(())
    }

    fn extract_tar_gz(
        &self,
        reader: std::io::Cursor<Vec<u8>>,
        extract_to: &str,
    ) -> Result<(), String> {
        print!("Attempting extraction... ");
        let _ = io::stdout().flush();

        let mut tar_gz_archive = GzDecoder::new(reader);

        // TODO: Extract selectively (instead of selectively moving in `install_font()`)
        Archive::new(&mut tar_gz_archive)
            .unpack(extract_to)
            .map_err(|e| {
                println_red!("Failed");
                e.to_string()
            })?;
        println_green!("Done");

        Ok(())
    }

    fn extract_tar_xz(
        &self,
        _reader: std::io::Cursor<Vec<u8>>,
        _extract_to: &str,
    ) -> Result<(), String> {
        todo!("XZ format is currently unsupported");

        // NOTE: `tar.xz` support is currently disabled due to
        // outdated `xz` crate dependencies for `zip` and `bzip2`

        // print!("Attempting extraction... ");
        // let _ = io::stdout().flush();

        // let mut tar_xz_archive = XzDecoder::new(reader);

        // Archive::new(&mut tar_xz_archive)
        //     .unpack(extract_to)
        //     .map_err(|e| {
        //         println_red!("Failed");
        //         e.to_string()
        //     })?;
        // println_green!("Done");

        // Ok(())
    }

    pub fn install_font(
        &self,
        args: &Args,
        installed_fonts: &mut InstalledFonts,
    ) -> Result<(), String> {
        let temp_dir = format!("{}/{}/{}/", cache_dir!(), &self.name, &self.tag);

        let dest_dir = installed_fonts
            .uninstall(&self.installer_name, args)?
            .unwrap_or_else(|| format!("{}/{}/", args.config.install_dir, &self.name));

        // Move the files specified by the installer into the target directory
        println!("Installing:");

        fs::create_dir_all(&dest_dir).map_err(|err| err.to_string())?;
        let mut errors = false;

        let mut files = Vec::new();
        visit_dirs(Path::new(&temp_dir), &mut |file| {
            let partial_path = &file.path().display().to_string().replace(&temp_dir, "");

            // Ignore files which don't satisfy the 'include' and 'exclude' patterns
            if !match_any_wildcard(partial_path, &self.include)
                || match_any_wildcard(partial_path, &self.exclude)
            {
                return;
            }

            let target_path = match self.keep_folders {
                true => {
                    if let Err(e) = fs::create_dir_all(
                        Path::new(&format!("{dest_dir}/{partial_path}"))
                            .parent()
                            .unwrap(),
                    ) {
                        println!("   {partial_path} ... {}", red!(&e.to_string()));
                        errors = true;
                        return;
                    }
                    partial_path
                }
                false => partial_path.split('/').next_back().unwrap(),
            };

            print!("   {target_path} ... ");
            let _ = io::stdout().flush();

            match fs::rename(
                format!("{temp_dir}/{partial_path}"),
                format!("{dest_dir}/{target_path}"),
            ) {
                Ok(()) => {
                    println_green!("Done");
                    files.push(target_path.to_owned());
                }
                Err(e) => {
                    println_red!("{e}");
                    errors = true;
                }
            }
        })
        .map_err(|e| e.to_string())?;

        match errors {
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

    #[must_use]
    pub fn has_updates(&self, installed_fonts: &InstalledFonts) -> bool {
        installed_fonts
            .installed
            .get(&self.installer_name)
            .is_none_or(|installed| self.url != installed.url.clone())
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
