use crate::font_page::FontPage;
use crate::installed::{InstalledFont, InstalledFonts};
use crate::wildcards::*;
use crate::Args;

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use reqwest::header::USER_AGENT;

use flate2::read::GzDecoder;
use tar::Archive;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Installer {
    pub name: String,
    #[serde(default)]
    tag: String,
    url: String,
    file: String,
    action: InstallAction,

    #[serde(default, skip_serializing)]
    installer_name: String,
    #[serde(default, skip_serializing)]
    download_buffer: Option<Vec<u8>>,
    #[serde(default, skip_serializing)]
    files: Vec<String>,
}

#[derive(Debug, Deserialize)]
enum InstallAction {
    Extract {
        include: Box<[String]>,
        exclude: Option<Box<[String]>>,
        keep_folders: Option<bool>,
    },
    SingleFile,
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

        if !match_wildcard(&installer.file, "*.*") {
            return Err(format!(
                "Installer for '{font_name}' did not specify a valid archive"
            ));
        }
        installer.file = installer.file.replace("$tag", &installer.tag);

        match installer.action {
            InstallAction::Extract {
                include,
                exclude,
                keep_folders,
            } => {
                installer.action = InstallAction::Extract {
                    include: include
                        .iter()
                        .map(|p| p.replace("$tag", &installer.tag))
                        .collect(),
                    exclude: exclude.map_or_else(
                        || None,
                        |p| {
                            Some(
                                p.iter()
                                    .map(|p| p.replace("$tag", &installer.tag))
                                    .collect(),
                            )
                        },
                    ),
                    keep_folders,
                }
            }
            InstallAction::SingleFile => (),
        }

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
                wildcard_substring(line, &(String::from("https://*") + &self.file), b"")
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

        print!("Downloading font... ");
        let _ = io::stdout().flush();

        // TODO: Show download progress
        let mut archive_buffer = Vec::new();
        remote_data.read_to_end(&mut archive_buffer).map_err(|e| {
            println_red!("Failed");
            e.to_string()
        })?;
        self.download_buffer = Some(archive_buffer);
        println_green!("Done");

        Ok(self)
    }

    pub fn prepare_install(&mut self) -> Result<&Self, String> {
        let data = self.download_buffer.take();
        if data.is_none() {
            return Err(format!(
                "{}: {}",
                self.installer_name,
                red!("Attempted extraction with no downloaded data"),
            ));
        }

        let extract_to = format!("{}/{}/", staging_dir!(), &self.name);
        let _ = fs::remove_dir_all(&extract_to);
        match &self.action {
            InstallAction::Extract {
                include,
                exclude,
                keep_folders,
            } => {
                let reader = std::io::Cursor::new(data.unwrap());
                match self.file.split('.').next_back() {
                    Some("zip") => {
                        self.files = Self::extract_zip(
                            reader,
                            &extract_to,
                            include,
                            &exclude.clone().unwrap_or_else(|| [].into()),
                            keep_folders.unwrap_or_default(),
                        )?;
                    }
                    Some("tar") => {}
                    Some("gz") => {
                        self.files = Self::extract_tar_gz(
                            reader,
                            &extract_to,
                            include,
                            &exclude.clone().unwrap_or_else(|| [].into()),
                            keep_folders.unwrap_or_default(),
                        )?;
                    }
                    Some("xz") => {
                        self.files = Self::extract_tar_xz(
                            reader,
                            &extract_to,
                            include,
                            &exclude.clone().unwrap_or_else(|| [].into()),
                            keep_folders.unwrap_or_default(),
                        )?;
                    }
                    Some(_) => return Err(format!("Unsupported archive extension: {}", self.file)),
                    None => {
                        return Err(format!("Archive requires a file extension: {}", self.file))
                    }
                }
            }
            InstallAction::SingleFile => {
                fs::create_dir_all(&extract_to).map_err(|e| e.to_string())?;
                let file = self.url.split('/').next_back().unwrap();
                self.files.push(file.to_string());
                fs::write(extract_to + file, data.unwrap()).map_err(|e| e.to_string())?;
            }
        }

        Ok(self)
    }

    fn extract_zip(
        reader: std::io::Cursor<Vec<u8>>,
        extract_to: &str,
        include: &[String],
        exclude: &[String],
        keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        print!("Attempting extraction... ");
        let _ = io::stdout().flush();

        let mut zip_archive = zip::ZipArchive::new(reader).map_err(|e| {
            println_red!("Failed to read the archive");
            e.to_string()
        })?;

        let files: Vec<String> = zip_archive
            .file_names()
            .map(ToString::to_string)
            .filter(|file| {
                // FIX: This creates all paths, regardless if they're included or not
                if file.ends_with('/') {
                    if keep_folders {
                        let _ = fs::create_dir_all(extract_to.to_owned() + file);
                    }
                    return false;
                }
                match_any_wildcard(file, include) && !match_any_wildcard(file, exclude)
            })
            .collect();
        fs::create_dir_all(extract_to).map_err(|e| {
            println_red!("{e}");
            e.to_string()
        })?;

        for file in &files {
            let mut file_contents = Vec::new();
            zip_archive
                .by_name(file)
                .map_err(|e| {
                    println_red!("{e}");
                    e.to_string()
                })?
                .read_to_end(&mut file_contents)
                .map_err(|e| {
                    println_red!("{e}");
                    e.to_string()
                })?;

            fs::write(
                extract_to.to_owned()
                    + match keep_folders {
                        true => file,
                        false => file.split('/').next_back().unwrap(),
                    },
                file_contents,
            )
            .map_err(|e| {
                println_red!("{e}");
                e.to_string()
            })?;
        }

        println_green!("Done");

        Ok(files)
    }

    fn extract_tar<R: io::Read>(
        mut archive: Archive<R>,
        extract_to: &str,
        include: &[String],
        exclude: &[String],
        keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        print!("Attempting extraction... ");
        fs::create_dir_all(extract_to).map_err(|e| {
            println_red!("{e}");
            e.to_string()
        })?;

        let mut fonts = Vec::new();
        for mut entry in archive.entries().map_err(|e| e.to_string())? {
            let entry = entry.as_mut().unwrap();
            let path = match keep_folders {
                true => entry.path().unwrap().to_string_lossy().into_owned(),
                false => entry
                    .path()
                    .unwrap()
                    .to_string_lossy()
                    .split('/')
                    .next_back()
                    .unwrap()
                    .to_string(),
            };

            if !match_any_wildcard(&path, include) || match_any_wildcard(&path, exclude) {
                continue;
            }
            if path.is_empty() || path.ends_with('/') {
                // FIX: This creates all paths, regardless if they're included or not
                fs::create_dir_all(extract_to.to_owned() + &path).map_err(|e| {
                    println_red!("{e}");
                    e.to_string()
                })?;
                continue;
            }

            let mut file_contents = Vec::new();
            entry.read_to_end(&mut file_contents).map_err(|e| {
                println_red!("{e}");
                e.to_string()
            })?;

            fs::write(&(extract_to.to_owned() + &path), file_contents)
                .map_err(|e| e.to_string())?;
            fonts.push(path);
        }

        println_green!("Done");

        Ok(fonts)
    }

    fn extract_tar_gz(
        reader: std::io::Cursor<Vec<u8>>,
        extract_to: &str,
        include: &[String],
        exclude: &[String],
        keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        let _ = io::stdout().flush();

        let mut tar_gz_archive = GzDecoder::new(reader);

        Self::extract_tar(
            Archive::new(&mut tar_gz_archive),
            extract_to,
            include,
            exclude,
            keep_folders,
        )
    }

    fn extract_tar_xz(
        _reader: std::io::Cursor<Vec<u8>>,
        _extract_to: &str,
        _include: &[String],
        _exclude: &[String],
        _keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        todo!("XZ format is currently unsupported");

        // NOTE: `tar.xz` support is currently disabled due to
        // outdated `xz` crate dependencies for `zip` and `bzip2`

        // let _ = io::stdout().flush();

        // let mut tar_xz_archive = XzDecoder::new(reader);

        // Self::extract_tar(
        //     Archive::new(&mut tar_gz_archive),
        //     extract_to,
        //     include,
        //     exclude,
        //     keep_folders,
        // )
    }

    pub fn finalize_install(
        &self,
        args: &Args,
        installed_fonts: &mut InstalledFonts,
    ) -> Result<(), String> {
        let staging_dir = format!("{}/{}/", staging_dir!(), &self.name);

        let target_dir = installed_fonts
            .uninstall(&self.installer_name, args)?
            .unwrap_or_else(|| format!("{}/{}/", args.config.install_dir, &self.name));

        // Move the files specified by the installer into the target directory
        println!("Installing:");

        fs::create_dir_all(&target_dir).map_err(|err| err.to_string())?;
        let mut errors = false;

        for file in &self.files {
            if let Err(e) =
                fs::create_dir_all(Path::new(&format!("{target_dir}/{file}")).parent().unwrap())
            {
                println!("   {file} ... {}", red!(&e.to_string()));
                errors = true;
                continue;
            }

            print!("   {file} ... ");
            let _ = io::stdout().flush();

            match fs::rename(
                format!("{staging_dir}/{file}"),
                format!("{target_dir}/{file}"),
            ) {
                Ok(()) => {
                    println_green!("Done");
                }
                Err(e) => {
                    println_red!("{e}");
                    errors = true;
                }
            }
        }

        match errors {
            false => {
                println!("Successfully installed {}", self.name);
                installed_fonts.update_entry(
                    &self.installer_name,
                    InstalledFont {
                        url: self.url.clone(),
                        dir: target_dir,
                        files: self.files.clone(),
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
