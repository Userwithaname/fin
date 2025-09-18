use crate::bar;
use crate::font_page::FontPage;
use crate::installed::{InstalledFont, InstalledFonts};
use crate::wildcards::*;
use crate::Args;

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::{self, stdout, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use reqwest::header::USER_AGENT;

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Installer {
    pub name: String,
    #[serde(default)]
    tag: String,
    pub url: String,
    file: String,
    check: Option<Checksum>,
    action: FileAction,

    #[serde(default, skip_serializing)]
    installer_name: String,
    #[serde(default, skip_serializing)]
    download_buffer: Option<Vec<u8>>,
    #[serde(default, skip_serializing)]
    files: Vec<String>,
    #[serde(skip_serializing)]
    font_page: Option<String>,
}

#[derive(Debug, Deserialize)]
enum Checksum {
    SHA256 { file: Option<String> },
}

#[derive(Debug, Deserialize)]
enum FileAction {
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
        installer_dir: String,
        installer_name: &str,
        override_version: Option<&str>,
        cached_pages: Arc<Mutex<HashMap<u64, FontPage>>>,
    ) -> Result<Self, String> {
        let mut installer: Self = toml::from_str(
            &fs::read_to_string(installer_dir + installer_name).map_err(|err| {
                eprintln!("Error reading the installer: {installer_name}");
                err.to_string()
            })?,
        )
        .map_err(|err| {
            eprintln!("Error parsing the installer: {installer_name}");
            err.to_string()
        })?;

        installer.installer_name = installer_name.to_string();
        Self::validate_name(&installer.name, installer_name)?;
        Self::validate_tag(&mut installer.tag, override_version);
        Self::validate_file(&mut installer.file, &installer.tag, installer_name)?;
        Self::validate_action(&mut installer.action, &installer.tag, installer_name)?;

        // For direct download links
        if installer.url.ends_with("$file") {
            // TODO: Get the redirected URL for direct links
            installer.url = installer.url.replace("$file", &installer.file);
            return Ok(installer);
        };

        let reqwest_client = reqwest::blocking::Client::new();
        installer.font_page = FontPage::get_font_page(
            &installer.url.replace("$tag", &installer.tag),
            Arc::clone(&args),
            &reqwest_client,
            cached_pages,
        )?
        .contents;
        installer.validate_url(installer_name)?;
        Ok(installer)
    }

    fn validate_name(name: &str, font_name: &str) -> Result<(), String> {
        if name.replace(['.', '/'], "").is_empty() || name.contains("..") {
            return Err(format!("{font_name}: Invalid name: \"{name}\""));
        }
        Ok(())
    }
    fn validate_tag(tag: &mut String, override_version: Option<&str>) {
        if let Some(version) = override_version {
            *tag = version.to_string();
        }
    }
    fn validate_file(file: &mut String, tag: &str, font_name: &str) -> Result<(), String> {
        if !match_wildcard(file, "*.*") {
            return Err(format!(
                "{font_name}: File must specify an extension: \"{file}\"",
            ));
        }
        if file.ends_with('*') {
            return Err(format!(
                "{font_name}: File must not end with a '*': \"{file}\"",
            ));
        }
        if file.len() < 2 {
            return Err(format!("{font_name}: Invalid file: \"{file}\"",));
        }
        *file = file.replace("$tag", tag);
        Ok(())
    }
    fn validate_action(action: &mut FileAction, tag: &str, font_name: &str) -> Result<(), String> {
        match action {
            FileAction::Extract {
                include, exclude, ..
            } => {
                if include.is_empty() {
                    return Err(format!("{font_name}: The include field must not be empty"));
                }
                *include = include.iter().map(|p| p.replace("$tag", tag)).collect();
                *exclude = exclude.as_ref().map_or_else(
                    || None,
                    |p| Some(p.iter().map(|p| p.replace("$tag", tag)).collect()),
                );
                Ok(())
            }
            FileAction::SingleFile => Ok(()),
        }
    }
    fn validate_url(&mut self, font_name: &str) -> Result<(), String> {
        if !match_wildcard(&self.url, "*://*.*/*") {
            return Err(format!("{font_name}: Invalid URL: \"{}\"", self.url));
        }
        self.url = Self::find_direct_link(
            &self.font_page.as_ref().unwrap(),
            &self.file,
            &self.installer_name,
        )?;
        Ok(())
    }

    /// Returns a direct link to the `file` found within `font_page`
    fn find_direct_link(
        font_page_contents: &str,
        file: &str,
        name: &str,
    ) -> Result<String, String> {
        font_page_contents
            .split('"')
            .find_map(|line| wildcard_substring(line, &(String::from("https://*") + file), b""))
            .map_or_else(
                || {
                    Err(format!(
                        "{name}: File \"{file}\" could not be found within the webpage"
                    ))
                },
                |link| Ok(link.to_string()),
            )
    }

    async fn obtain_checksum(&mut self, reqwest_client: &reqwest::Client) -> Result<(), String> {
        match &mut self.check {
            Some(Checksum::SHA256 { file }) => {
                if file.is_none() {
                    *file = Some(self.font_page.take().unwrap());
                    return Ok(());
                }

                let file = file.as_mut().unwrap();
                Self::validate_file(file, &self.tag, &self.installer_name)?;
                let file_link = Self::find_direct_link(
                    &self.font_page.take().unwrap(),
                    file,
                    &self.installer_name,
                )?;
                *file = reqwest_client
                    .get(&file_link)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .text()
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    pub async fn download_font(&mut self) -> Result<&mut Self, String> {
        let reqwest_client = reqwest::Client::new();

        self.obtain_checksum(&reqwest_client).await?;

        let remote_data = reqwest_client
            .get(&self.url)
            .header(USER_AGENT, "fin")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let filename = &self.url.split('/').next_back().unwrap_or_default();

        // TODO: Show download progress
        print!(
            "… Downloading: {filename} ({})",
            Self::format_size(remote_data.content_length().unwrap_or_default() as f64)
        );
        let _ = stdout().flush();

        let archive_buffer = remote_data.bytes().await.map_err(|e| {
            println!("\r{} Downloading: {filename}", red!("×"));
            e.to_string()
        })?;
        self.download_buffer = Some(archive_buffer.to_vec());
        println!("\r{} Downloading: {filename}", green!("✓"));

        Ok(self)
    }

    pub fn verify_download(&mut self) -> Result<&mut Self, String> {
        let data = self.download_buffer.as_ref().unwrap();

        match self.check.take() {
            Some(Checksum::SHA256 { file }) => {
                let filename = &self.url.split('/').next_back().unwrap_or_default();
                print!("… Verifying:   {filename}");
                let _ = stdout().flush();

                let mut hasher = Sha256::new();
                hasher.write_all(data).map_err(|e| e.to_string())?;
                let sum = hasher.finalize();

                match file.as_ref().unwrap().contains(&format!("{sum:x}")) {
                    true => {
                        println!("\r{} Verifying:   {filename}", green!("✓"));
                        Ok(self)
                    }
                    false => {
                        println!("\r{} Verifying:   {filename}", red!("×"));
                        Err(format!("{filename}: Integrity check failed"))
                    }
                }
            }
            None => Ok(self),
        }
    }

    fn format_size(mut num_bytes: f64) -> String {
        const UNITS: &[&str] = &["bytes", "KB", "MB", "GB", "TB"];
        let mut unit_index = 0;
        while num_bytes > 1024.0 {
            num_bytes /= 1024.0;
            unit_index += 1;
        }
        format!("{num_bytes:.1} {}", UNITS[unit_index])
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

        let extract_to = staging_dir!() + &self.name + "/";
        let _ = fs::remove_dir_all(&extract_to);
        match &mut self.action {
            FileAction::Extract {
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
                            &exclude.take().unwrap_or_else(|| [].into()),
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
                            &exclude.take().unwrap_or_else(|| [].into()),
                            keep_folders.unwrap_or_default(),
                        )?;
                    }
                    Some("xz") => {
                        self.files = Self::extract_tar_xz(
                            args,
                            reader,
                            &extract_to,
                            include,
                            &exclude.take().unwrap_or_else(|| [].into()),
                            keep_folders.unwrap_or_default(),
                        )?;
                    }
                    Some(_) => return Err(format!("Unsupported archive extension: {}", self.file)),
                    None => {
                        return Err(format!("Archive requires a file extension: {}", self.file))
                    }
                }
            }
            FileAction::SingleFile => {
                let verbose = args.options.verbose || args.config.verbose_files;
                fs::create_dir_all(&extract_to).map_err(|e| e.to_string())?;
                let file = self.url.split('/').next_back().unwrap();

                let update_progress_bar = |status_symbol: &str, progress: f64| {
                    bar::show_progress(
                        &format!("{status_symbol} Staging:    "),
                        progress,
                        &format!(" {progress} / 1"),
                    );
                };

                match verbose {
                    true => {
                        println!("Staging:");
                        print!("   {file} ... ");
                        let _ = stdout().flush();
                    }
                    false => update_progress_bar("…", 0.0),
                }

                self.files.push(file.to_string());
                fs::write(extract_to + file, data.unwrap()).map_err(|e| {
                    update_progress_bar(&green!("✓"), 1.0);
                    println_red!("{e}");
                    e.to_string()
                })?;

                match verbose {
                    true => println_green!("Done"),
                    false => update_progress_bar(&green!("✓"), 1.0),
                }
                println!();
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

        let mut file_count = 0.0;
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

        let update_progress_bar = |status_symbol: &str, files_processed: f64| {
            bar::show_progress(
                &format!("{status_symbol} Staging:    "),
                files_processed / file_count,
                &format!(" {files_processed} / {file_count}"),
            );
        };

        let mut files_processed = 0.0;

        for file in &mut files {
            files_processed += 1.0;

            match verbose {
                true => {
                    print!("   {file} ... ");
                    let _ = stdout().flush();
                }
                false => update_progress_bar("…", files_processed),
            }

            // FIX: No such file or directory error when using `keep_folders`

            let mut file_contents = Vec::new();
            zip_archive
                .by_name(file)
                .map_err(|e| {
                    match verbose {
                        true => println_red!("{e}"),
                        false => {
                            update_progress_bar(&red!("×"), files_processed);
                            println!();
                        }
                    }
                    e.to_string()
                })?
                .read_to_end(&mut file_contents)
                .map_err(|e| {
                    match verbose {
                        true => println_red!("{e}"),
                        false => {
                            update_progress_bar(&red!("×"), files_processed);
                            println!();
                        }
                    }
                    e.to_string()
                })?;

            if !keep_folders {
                *file = file.split('/').next_back().unwrap().to_owned();
            }

            fs::write(extract_to.to_owned() + file, file_contents).map_err(|e| {
                match verbose {
                    true => println_red!("{e}"),
                    false => {
                        update_progress_bar(&red!("×"), files_processed);
                        println!();
                    }
                }
                e.to_string()
            })?;

            if verbose {
                println_green!("Done");
            }
        }

        if !verbose {
            update_progress_bar(&green!("✓"), files_processed);
            println!();
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
                let _ = stdout().flush();
            }
        }

        fs::create_dir_all(extract_to).map_err(|e| {
            println!("   Directory creation error: {}", format_red!("{e}"));
            e.to_string()
        })?;

        let update_progress_bar = |status_symbol: &str, files_processed: f64| {
            bar::show_progress(
                &format!("{status_symbol} Staging:    "),
                1.0,
                &format!(" {files_processed} / {files_processed}"),
            );
        };

        let mut files_processed = 0.0;
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
                    let _ = stdout().flush();
                }
                false => {
                    files_processed += 1.0;
                    update_progress_bar("…", files_processed);
                }
            }

            if file.is_empty() || file.ends_with('/') {
                if keep_folders {
                    // NOTE: This creates all paths, regardless if they're included or not
                    fs::create_dir_all(extract_to.to_owned() + &file).map_err(|e| {
                        match verbose {
                            true => println_red!("{e}"),
                            false => {
                                update_progress_bar(&red!("×"), files_processed);
                                println!("\n{file}: {}", format_red!("{e}"));
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
                        update_progress_bar(&red!("×"), files_processed);
                        println!("\n{file}: {}", format_red!("{e}"));
                    }
                }
                e.to_string()
            })?;

            fs::write(&(extract_to.to_owned() + &file), file_contents).map_err(|e| {
                if !verbose {
                    update_progress_bar(&red!("×"), files_processed);
                    println!("\n{file}: {}", format_red!("{e}"));
                }
                e.to_string()
            })?;
            fonts.push(file);

            if verbose {
                println_green!("Done");
            }
        }

        if !verbose {
            update_progress_bar(&green!("✓"), files_processed);
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
        let _ = stdout().flush();

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

        // let _ = stdout().flush();

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
        let (target_dir, old_files) = &installed_fonts
            .lock()
            .unwrap()
            .installed
            .get(&self.installer_name)
            .map_or_else(
                || (format!("{}/{}/", args.config.install_dir, &self.name), None),
                |installed_font| {
                    (
                        installed_font.dir.clone(),
                        Some(installed_font.files.clone()),
                    )
                },
            );

        fs::create_dir_all(target_dir).map_err(|err| err.to_string())?;

        match verbose {
            true => println!("Installing:"),
            false => {
                print!("Installing:");
                let _ = stdout().flush();
            }
        }

        let update_progress_bar = |status_symbol: &str, files_processed: f64| {
            bar::show_progress(
                &format!("{status_symbol} Installing: "),
                files_processed / self.files.len() as f64,
                &format!(" {files_processed} / {}", self.files.len()),
            );
        };

        let mut errors = false;
        let mut files_processed = 0.0;

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

            files_processed += 1.0;
            match verbose {
                true => {
                    print!("   {file} ... ");
                    let _ = stdout().flush();
                }
                false => update_progress_bar("…", files_processed),
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
                    update_progress_bar(&green!("✓"), files_processed);
                    println!();
                }

                installed_fonts
                    .lock()
                    .unwrap()
                    .update_entry(
                        &self.installer_name,
                        InstalledFont {
                            url: self.url.clone(),
                            dir: target_dir.to_string(),
                            files: self.files.clone(),
                        },
                    )
                    .cleanup(args, &self.installer_name, old_files.as_ref())
                    .map_err(|()| "Failed to cleanup")?;
            }
            true => {
                update_progress_bar(&red!("×"), files_processed);
                println!("\nErrors were encountered while installing {}", self.name)
            }
        }

        Ok(())
    }

    /// Returns the installer names of all available installers matched
    /// by any of the provided filter patterns
    pub fn filter_installers(filters: &[String]) -> Result<Vec<String>, String> {
        let installers_dir = installers_dir!();
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

    #[must_use]
    pub fn has_updates(&self, installed_fonts: &Arc<Mutex<InstalledFonts>>) -> bool {
        installed_fonts
            .lock()
            .unwrap()
            .installed
            .get(&self.installer_name)
            .is_none_or(|installed| {
                self.url != installed.url || !fs::exists(&installed.dir).unwrap_or_default()
            })
    }
}
