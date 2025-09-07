use crate::bar;
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
    pub url: String,
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

        if !match_wildcard(&installer.file, "*.*") {
            return Err(format!(
                "Installer for '{font_name}' did not specify a valid archive"
            ));
        }
        installer.file = installer.file.replace("$tag", &installer.tag);

        let reqwest_client = reqwest::blocking::Client::new();
        installer.url = match installer.url.ends_with("$file") {
            // TODO: Get the redirected URL for direct links
            true => installer.url.replace("$file", &installer.file),
            false => installer.find_direct_link(FontPage::get_font_page(
                &installer.url.replace("$tag", &installer.tag),
                args,
                &reqwest_client,
                cached_pages,
            )?)?,
        };

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
    pub fn filter_installed(
        filters: &[String],
        installed_fonts: &Arc<Mutex<InstalledFonts>>,
    ) -> Vec<String> {
        let installed_fonts = installed_fonts.lock().unwrap().get_names();

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

        print!("… Awaiting response");
        let _ = io::stdout().flush();

        let mut remote_data = reqwest_client
            .get(&self.url)
            .header(USER_AGENT, "fin")
            .send()
            .map_err(|e| {
                println!("\r{} Awaiting response", red!("×"));
                e.to_string()
            })?;
        println!("\r{} Awaiting response", green!("✓"));

        let filename = &self.url.split('/').next_back().unwrap_or_default();

        // TODO: Show download progress
        print!("… Downloading: {filename}");
        let _ = io::stdout().flush();

        let mut archive_buffer = Vec::new();
        remote_data.read_to_end(&mut archive_buffer).map_err(|e| {
            println!("\r{} Downloading: {filename}", red!("×"));
            e.to_string()
        })?;
        self.download_buffer = Some(archive_buffer);
        println!("\r{} Downloading: {filename}", green!("✓"));

        Ok(self)
    }

    pub fn prepare_install(&mut self, args: &Args) -> Result<&Self, String> {
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
                            args,
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
                            args,
                            reader,
                            &extract_to,
                            include,
                            &exclude.clone().unwrap_or_else(|| [].into()),
                            keep_folders.unwrap_or_default(),
                        )?;
                    }
                    Some("xz") => {
                        self.files = Self::extract_tar_xz(
                            args,
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
                let verbose = args.options.verbose || args.config.verbose_files;
                fs::create_dir_all(&extract_to).map_err(|e| e.to_string())?;
                let file = self.url.split('/').next_back().unwrap();

                match verbose {
                    true => {
                        println!("Staging:");
                        print!("   {file} ... ");
                        let _ = io::stdout().flush();
                    }
                    false => bar::show_progress("… Staging:    ", 0.0, " 0 / 1"),
                }

                self.files.push(file.to_string());
                fs::write(extract_to + file, data.unwrap()).map_err(|e| {
                    bar::show_progress(&format!("{} Staging:    ", green!("✓")), 1.0, " 1 / 1\n");
                    println_red!("{e}");
                    e.to_string()
                })?;

                match verbose {
                    true => println_green!("Done"),
                    false => bar::show_progress(
                        &format!("{} Staging:    ", green!("✓")),
                        1.0,
                        " 1 / 1\n",
                    ),
                }
            }
        }

        Ok(self)
    }

    fn extract_zip(
        args: &Args,
        reader: std::io::Cursor<Vec<u8>>,
        extract_to: &str,
        include: &[String],
        exclude: &[String],
        keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        let verbose = args.options.verbose || args.config.verbose_files;
        match verbose {
            true => println!("Staging:"),
            false => print!("Staging:"),
        }
        let mut zip_archive = zip::ZipArchive::new(reader).map_err(|e| {
            println_red!("Failed to read the archive");
            e.to_string()
        })?;

        let mut progress = 0;
        let mut file_count = 0f64;
        let mut files: Vec<String> = zip_archive
            .file_names()
            .map(ToString::to_string)
            .filter(|file| {
                if file.ends_with('/') {
                    if keep_folders {
                        // NOTE: This creates all paths, regardless if they're included or not
                        let _ = fs::create_dir_all(extract_to.to_owned() + file).inspect_err(|e| {
                            println!("   Directory creation error: {}", format_red!("{e}"));
                        });
                    }
                    return false;
                }
                if match_any_wildcard(file, include) && !match_any_wildcard(file, exclude) {
                    file_count += 1.0;
                    true
                } else {
                    false
                }
            })
            .collect();

        fs::create_dir_all(extract_to).map_err(|e| {
            println!("   Directory creation error: {}", format_red!("{e}"));
            e.to_string()
        })?;

        for file in &mut files {
            progress += 1;
            match verbose {
                true => {
                    print!("   {file} ... ");
                    let _ = io::stdout().flush();
                }
                false => bar::show_progress(
                    "… Staging:    ",
                    progress as f64 / file_count,
                    &format!(" {progress} / {file_count}"),
                ),
            }

            let mut file_contents = Vec::new();
            zip_archive
                .by_name(file)
                .map_err(|e| {
                    match verbose {
                        true => println_red!("{e}"),
                        false => {
                            bar::show_progress(
                                &format!("{} Staging:   ", red!("×")),
                                progress as f64 / file_count,
                                &format!(" {progress} / {file_count}\n"),
                            );
                        }
                    }
                    e.to_string()
                })?
                .read_to_end(&mut file_contents)
                .map_err(|e| {
                    match verbose {
                        true => println_red!("{e}"),
                        false => {
                            bar::show_progress(
                                &format!("{} Installing: ", red!("×")),
                                progress as f64 / file_count,
                                &format!(" {progress} / {file_count}\n"),
                            );
                        }
                    }
                    e.to_string()
                })?;

            if !keep_folders {
                *file = file.split('/').next_back().unwrap().to_owned();
            }

            fs::write(extract_to.to_owned() + file, file_contents).map_err(|e| {
                if !verbose {
                    bar::show_progress(
                        &format!("{} Staging:    ", red!("×")),
                        progress as f64 / file_count,
                        &format!(" {progress} / {file_count}\n"),
                    );
                }
                println_red!("{e}");
                e.to_string()
            })?;

            if verbose {
                println_green!("Done");
            }
        }

        if !verbose {
            bar::show_progress(
                &format!("{} Staging:    ", green!("✓")),
                1.0,
                &format!(" {progress} / {file_count}\n"),
            );
        }

        Ok(files)
    }

    fn extract_tar<R: io::Read>(
        args: &Args,
        mut archive: Archive<R>,
        extract_to: &str,
        include: &[String],
        exclude: &[String],
        keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        let verbose = args.options.verbose || args.config.verbose_files;
        match verbose {
            true => println!("Staging:"),
            false => {
                print!("Staging:");
                let _ = io::stdout().flush();
            }
        }

        fs::create_dir_all(extract_to).map_err(|e| {
            println!("   Directory creation error: {}", format_red!("{e}"));
            e.to_string()
        })?;

        let mut progress = 0;
        let mut fonts = Vec::new();
        let entries = archive.entries().map_err(|e| e.to_string())?;
        for mut entry in entries {
            let entry = entry.as_mut().unwrap();
            let file = match keep_folders {
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

            if !match_any_wildcard(&file, include) || match_any_wildcard(&file, exclude) {
                continue;
            }

            match verbose {
                true => {
                    print!("   {file} ... ");
                    let _ = io::stdout().flush();
                }
                false => {
                    progress += 1;
                    bar::show_progress("… Staging:    ", 1.0, &format!(" {progress} / {progress}"));
                }
            }

            if file.is_empty() || file.ends_with('/') {
                if keep_folders {
                    // NOTE: This creates all paths, regardless if they're included or not
                    fs::create_dir_all(extract_to.to_owned() + &file).map_err(|e| {
                        match verbose {
                            true => println_red!("{e}"),
                            false => {
                                bar::show_progress(
                                    &format!("{} Staging:    ", red!("×")),
                                    1.0,
                                    &format!(" {progress} / {progress}\n"),
                                );
                                println!("{file}: {}", format_red!("{e}"));
                            }
                        }
                        e.to_string()
                    })?;
                }
                continue;
            }

            let mut file_contents = Vec::new();
            entry.read_to_end(&mut file_contents).map_err(|e| {
                match verbose {
                    true => println_red!("{e}"),
                    false => {
                        bar::show_progress(
                            &format!("{} Staging:    ", red!("×")),
                            1.0,
                            &format!(" {progress} / {progress}\n"),
                        );
                        println!("{file}: {}", format_red!("{e}"));
                    }
                }
                e.to_string()
            })?;

            fs::write(&(extract_to.to_owned() + &file), file_contents).map_err(|e| {
                if !verbose {
                    bar::show_progress(
                        "… Staging:    ",
                        1.0,
                        &format!("{} {progress} / {progress}\n", red!("×")),
                    );
                    println!("{file}: {}", format_red!("{e}"));
                }
                e.to_string()
            })?;
            fonts.push(file);

            if verbose {
                println_green!("Done");
            }
        }

        if !verbose {
            bar::show_progress(
                &format!("{} Staging:    ", green!("✓")),
                1.0,
                &format!(" {progress} / {progress}"),
            );
            println!();
        }

        Ok(fonts)
    }

    fn extract_tar_gz(
        args: &Args,
        reader: std::io::Cursor<Vec<u8>>,
        extract_to: &str,
        include: &[String],
        exclude: &[String],
        keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        let _ = io::stdout().flush();

        let mut tar_gz_archive = GzDecoder::new(reader);

        Self::extract_tar(
            args,
            Archive::new(&mut tar_gz_archive),
            extract_to,
            include,
            exclude,
            keep_folders,
        )
    }

    fn extract_tar_xz(
        _args: &Args,
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
        //     args,
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
        installed_fonts: &Arc<Mutex<InstalledFonts>>,
    ) -> Result<(), String> {
        let verbose = args.options.verbose | args.config.verbose_files;
        let staging_dir = format!("{}/{}/", staging_dir!(), &self.name);
        let target_dir = installed_fonts
            .lock()
            .unwrap()
            .uninstall(&self.installer_name, args, false)?
            .unwrap_or_else(|| format!("{}/{}/", args.config.install_dir, &self.name));

        fs::create_dir_all(&target_dir).map_err(|err| err.to_string())?;

        match verbose {
            true => println!("Installing:"),
            false => {
                print!("Installing:");
                let _ = io::stdout().flush();
            }
        }

        let mut errors = false;
        let mut progress = 0;

        // Move the files specified by the installer into the target directory
        for file in &self.files {
            if let Err(e) =
                fs::create_dir_all(Path::new(&format!("{target_dir}/{file}")).parent().unwrap())
            {
                match verbose {
                    true => println!("   {file} ... {}", format_red!("{e}")),
                    false => println!("\n{file}: {}", format_red!("{e}")),
                }
                errors = true;
                continue;
            }

            progress += 1;
            match verbose {
                true => {
                    print!("   {file} ... ");
                    let _ = io::stdout().flush();
                }
                false => {
                    bar::show_progress(
                        "… Installing: ",
                        f64::from(progress) / self.files.len() as f64,
                        &format!(" {progress} / {}", self.files.len()),
                    );
                }
            }

            match fs::rename(
                format!("{staging_dir}/{file}"),
                format!("{target_dir}/{file}"),
            ) {
                Ok(()) => {
                    if verbose {
                        println_green!("Done");
                    }
                }
                Err(e) => {
                    if verbose {
                        println_red!("{e}");
                    }
                    errors = true;
                }
            }
        }

        match errors {
            false => {
                if !verbose {
                    bar::show_progress(
                        &format!("{} Installing: ", green!("✓")),
                        1.0,
                        &format!(" {progress} / {}\n", &self.files.len()),
                    );
                }

                installed_fonts.lock().unwrap().update_entry(
                    &self.installer_name,
                    InstalledFont {
                        url: self.url.clone(),
                        dir: target_dir,
                        files: self.files.clone(),
                    },
                );
            }
            true => {
                bar::show_progress(
                    &format!("{} Installing: ", red!("×")),
                    1.0,
                    &format!(" {progress} / {}\n", &self.files.len()),
                );
                println!("Errors were encountered while installing {}", self.name)
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn has_updates(&self, installed_fonts: &Arc<Mutex<InstalledFonts>>) -> bool {
        installed_fonts
            .lock()
            .unwrap()
            .installed
            .get(&self.installer_name)
            .is_none_or(|installed| self.url != installed.url.clone())
    }
}
