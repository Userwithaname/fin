use crate::installed::{InstalledFont, InstalledFonts};
use crate::wildcards::*;
use crate::Args;

use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, DirEntry};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct FontPage {
    time: u64,
    contents: Option<String>,
}

impl Default for FontPage {
    fn default() -> Self {
        Self {
            time: 0,
            contents: None,
        }
    }
}

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
    ) -> Result<Self, String> {
        let mut installer: Self = toml::from_str(
            &fs::read_to_string(format!(
                "{}/.config/fin/installers/{}",
                env::var("HOME").unwrap(),
                font_name
            ))
            .map_err(|err| {
                eprintln!(
                    "Error: Could not read the installer file for '{}'",
                    font_name
                );
                err.to_string()
            })?,
        )
        .map_err(|err| {
            eprintln!("Could not parse the installer for '{}'", font_name);
            err.to_string()
        })?;

        installer.installer_name = font_name.to_string();

        if let Some(version) = override_version {
            installer.tag = version.to_string();
        }

        if !match_any_wildcard(&installer.url, &["*://*.*/*".to_string()]) {
            return Err(format!(
                "Installer for '{}' did not specify a valid URL",
                font_name
            ));
        }
        installer.url = installer.url.replace("$tag", &installer.tag);

        let reqwest_client = reqwest::blocking::Client::new();
        installer.url_to_direct_link(&args, &reqwest_client)?;

        if !match_any_wildcard(&installer.archive, &["*.*".to_string()]) {
            return Err(format!(
                "Installer for '{}' did not specify a valid archive",
                font_name
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

    pub fn find_installers(filter: &[String]) -> Result<Vec<String>, String> {
        let installers_dir = format!("{}/.config/fin/installers", env::var("HOME").unwrap());
        if !Path::new(&installers_dir).exists() {
            return Err(format!(
                "Installers directory does not exist: {}",
                installers_dir
            ));
        }
        // TODO: Make the 'font-name:version' format work again
        let installers = fs::read_dir(installers_dir).map_err(|e| e.to_string())?;
        Ok(installers
            .filter_map(|installer| {
                installer.ok().and_then(|e| {
                    e.path()
                        .file_name()
                        .and_then(|n| n.to_str().map(|s| String::from(s)))
                })
            })
            .filter(|installer| match_any_wildcard(installer, filter))
            .collect())
    }

    pub fn find_installed(
        filter: &[String],
        installed_fonts: &mut InstalledFonts,
    ) -> Result<Vec<String>, String> {
        // TODO: Make the 'font-name:version' format work again
        let installed_fonts = installed_fonts.get_names();
        Ok(installed_fonts
            .iter()
            .filter_map(|font| {
                if match_any_wildcard(font, filter) {
                    return Some(font.clone());
                }
                None
            })
            .collect())
    }

    /// Replaces `self.url` with a direct link to the font archive
    /// Prior to calling this function, the URL is expected to lead to a webpage,
    /// which has the direct link discoverable in plain text within its source.
    pub fn url_to_direct_link(
        &mut self,
        args: &Args,
        client: &reqwest::blocking::Client,
    ) -> Result<(), String> {
        let mut hasher = DefaultHasher::new();
        self.url.hash(&mut hasher);
        let cache_file = format!(
            "{}/.cache/fin/{}",
            env::var("HOME").unwrap(),
            hasher.finish()
        );

        //TODO: Memory caching: remember cached files so they don't have to be
        //      read from the disk multiple times, if installing multiple fonts
        //      from the same source (and to avoid re-downloading and re-writing
        //      the cache multiple times when using --refresh)
        let mut cache: FontPage =
            toml::from_str(&fs::read_to_string(&cache_file).unwrap_or_default())
                .unwrap_or_default();
        let system_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .div_f64(60.0)
            .as_secs();

        if cache.contents.is_none()
            || args.options.refresh
            || system_time.wrapping_sub(cache.time) >= args.config.cache_timeout
        {
            if args.options.verbose {
                println!("Updating cache: {} ({})", cache_file, self.url);
            }
            let page = client
                .get(&self.url)
                .header(USER_AGENT, "fin")
                .send()
                .map_err(|e| {
                    // eprintln!("Could not access the URL for '{}'", self.name);
                    e.to_string()
                })?;
            // let page_url = page.url().clone();

            cache.time = system_time;
            cache.contents = Some(page.text().map_err(|e| {
                eprintln!(
                    "Could not determine the font archive URL for '{}'",
                    self.name
                );
                e.to_string()
            })?);

            fs::write(
                &cache_file,
                &toml::to_string(&cache).map_err(|e| {
                    eprintln!("Failed to serialize cache: {}", &cache_file);
                    e.to_string()
                })?,
            )
            .map_err(|e| {
                eprint!("Failed to write cache file to disk: {}", &cache_file);
                e.to_string()
            })?;
        }

        self.url = cache
            .contents
            .unwrap()
            .split('"')
            .filter_map(|line| {
                wildcard_substring(line, &(String::from("https://*") + &self.archive), b"\"")
            })
            .next()
            .expect("Archive download link not found") // TODO: Error handling
            // .ok_or(String::from("Archive download link not found"))?
            .to_string();

        Ok(())
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
            format!(
                "{}/.cache/fin/{}/{}/",
                env::var("HOME").map_err(|e| e.to_string())?,
                &self.name,
                &self.tag
            ),
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
        let temp_dir = format!(
            "{}/.cache/fin/{}/{}/",
            env::var("HOME").map_err(|err| err.to_string())?,
            &self.name,
            &self.tag,
        );

        let dest_dir = match installed_fonts.installed.get(&self.name) {
            Some(installed) => installed.dir.clone(),
            None => format!("{}/{}/", args.config.install_dir, &self.name),
        };

        // Move the files specified by the installer into the target directory
        println!("Installing:");

        fs::create_dir_all(&dest_dir).map_err(|err| err.to_string())?;
        let errors = Arc::new(Mutex::new(false));

        visit_dirs(Path::new(&temp_dir), &|file| {
            let partial_path = &file.path().display().to_string().replace(&temp_dir, "");

            // Ignore files which don't satisfy the 'include' and 'exclude' patterns
            if !match_any_wildcard(partial_path, &self.include)
                || match_any_wildcard(partial_path, &self.exclude)
            {
                return;
            }

            print!("   {} ... ", partial_path);

            // TODO: Option to preserve directory structure? (specified by the installer)
            // if let Err(e) = fs::create_dir_all(
            //     Path::new(&format!("{}/{}", &dest_dir, &partial_path))
            //         .parent()
            //         .unwrap(),
            // ) {
            //     println!("{}{}{}", "\x1b[91m", e.to_string(), "\x1b[0m");
            //     *errors.lock().unwrap() = true;
            //     return;
            // }

            match fs::rename(
                format!("{}/{}", temp_dir, partial_path),
                format!("{}/{}", dest_dir, partial_path.split('/').last().unwrap()),
                // format!("{}/{}", dest_dir, partial_path), // <-- to preserve subdirectories
            ) {
                Ok(_) => println!("{}Done{}", "\x1b[92m", "\x1b[0m"),
                Err(e) => {
                    println!("{}{}{}", "\x1b[91m", e.to_string(), "\x1b[0m");
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
fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry)) -> io::Result<()> {
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
