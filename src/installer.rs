use crate::bar::ProgressBar;
use crate::checksum::Checksum;
use crate::file_action::FileAction;
use crate::font_page::FontPage;
use crate::installed::{InstalledFont, InstalledFonts};
use crate::paths::{collapse_home, installers_dir, staging_dir};
use crate::source::Source;
use crate::Args;
use crate::{format_size, wildcards::*};

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::{self, stdout, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use futures::stream::StreamExt;
use reqwest::header::USER_AGENT;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Installer {
    pub name: String,
    pub source: Source,
    pub action: FileAction,
    check: Option<Checksum>,

    #[serde(default, skip_serializing)]
    pub installer_name: String,

    // TODO: Re-think how the below fields are stored
    #[serde(default, skip_serializing)]
    pub data: Option<Vec<u8>>,
    #[serde(default, skip_serializing)]
    pub data_size: f64,
    #[serde(default, skip_serializing)]
    pub files: Vec<String>,
    #[serde(skip_serializing)]
    pub font_page: Option<String>,
}

impl Installer {
    pub fn parse(
        args: &Arc<Args>,
        installer_dir: &str,
        installer_name: &str,
        override_version: Option<&str>,
        cached_pages: Arc<Mutex<HashMap<String, FontPage>>>,
    ) -> Result<Self, String> {
        let mut installer: Self = toml::from_str(
            &fs::read_to_string([installer_dir, installer_name].concat()).map_err(|err| {
                eprintln!("Error reading installer: {installer_name}");
                err.to_string()
            })?,
        )
        .map_err(|err| {
            eprintln!("Error parsing installer: {installer_name}");
            err.to_string()
        })?;

        installer.installer_name = installer_name.to_string();
        Self::validate_name(&installer.name, installer_name)?;

        installer.source = {
            let mut source = installer.source.take();
            source.validate_tag(override_version);
            installer
                .action
                .validate(source.ref_tag()?, installer_name)?;
            source.validate(installer.action.ref_file()?, installer_name)?;
            source.into_direct_url(&mut installer, args, cached_pages)?;
            source
        };

        Ok(installer)
    }

    fn validate_name(name: &str, font_name: &str) -> Result<(), String> {
        if name.replace(['.', '/'], "").is_empty() || name.contains("..") {
            return Err(format!("{font_name}: Invalid name: \"{name}\""));
        }
        Ok(())
    }

    /// Downloads the font and stores its contents in `self.data`
    pub async fn download_font(&mut self) -> Result<&mut Self, String> {
        let mut progress_bar = ProgressBar::new("Downloading:");
        let reqwest_client = reqwest::Client::new();

        print!("… Downloading…",);
        let _ = stdout().flush();

        let font_page = self.font_page.take();
        if let Some(checksum) = &mut self.check {
            checksum
                .obtain(
                    font_page,
                    self.source.ref_tag()?,
                    &reqwest_client,
                    &self.installer_name,
                )
                .await
                .inspect_err(|_| progress_bar.fail())?;
        }

        let remote_data = reqwest_client
            .get(self.source.ref_direct_url()?)
            .header(USER_AGENT, "fin")
            .send()
            .await
            .map_err(|e| {
                progress_bar.fail();
                e.to_string()
            })?;

        self.data_size = remote_data.content_length().unwrap_or_default() as f64;
        let file_size = format_size(self.data_size);

        let mut downloaded_bytes = 0;

        let mut buffer = Vec::new();
        let mut stream = remote_data.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                progress_bar.fail();
                e.to_string()
            })?;

            io::copy(&mut chunk.as_ref(), &mut buffer).map_err(|e| e.to_string())?;

            downloaded_bytes += chunk.len();

            let downloaded_bytes = downloaded_bytes as f64;
            if downloaded_bytes > self.data_size {
                self.data_size = downloaded_bytes;
            }

            let progress_text = format_size(downloaded_bytes);
            progress_bar.update_progress(
                downloaded_bytes / self.data_size,
                &format!(" {progress_text} / {file_size}"),
            );
        }

        self.data = Some(buffer);

        progress_bar.pass();

        Ok(self)
    }

    /// Verifies dowbloaded data integrity using a checksum
    pub fn verify_download(&mut self) -> Result<&mut Self, String> {
        let data = self.data.as_ref().unwrap().as_slice();
        match self.check.take() {
            Some(mut checksum) => checksum
                .check(data, self.data_size, &self.source)
                .map(|()| self),
            None => Ok(self),
        }
    }

    /// Prepares the font for installation by writing its
    /// files to a staging directory (`paths::staging_dir`)
    pub fn prepare_install(&mut self, args: &Args) -> Result<&Self, String> {
        let Some(data) = self.data.take() else {
            return Err(format!(
                "{}: {}",
                self.installer_name,
                red!("No data downloaded"),
            ));
        };

        let extract_to = [staging_dir(), &self.name, "/"].concat();
        let _ = fs::remove_dir_all(&extract_to);

        self.action
            .take()
            .stage_install(self, data, extract_to, args)?;
        Ok(self)
    }

    /// Moves the files from `paths::staging_dir` into the installation directory
    pub fn finalize_install(
        &self,
        args: &Args,
        installed_fonts: &Arc<Mutex<InstalledFonts>>,
    ) -> Result<(), String> {
        let verbose = args.options.verbose | args.config.verbose_files;

        let staging_dir = format!("{}/{}/", staging_dir(), &self.name);
        let (target_dir, old_files) = &installed_fonts
            .lock()
            .unwrap()
            .installed
            .get(&self.installer_name)
            .map_or_else(
                || (format!("{}/{}/", args.config.install_dir, &self.name), None),
                |installed| (installed.get_dir(), Some(installed.files.clone())),
            );

        fs::create_dir_all(target_dir).map_err(|err| err.to_string())?;

        match verbose {
            true => println!("Installing:"),
            false => {
                print!("… Installing…");
                let _ = stdout().flush();
            }
        }

        let mut errors = false;
        let mut files_processed = 0.0;
        let mut progress_bar = ProgressBar::new("Installing:");

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
                false => progress_bar.update_progress(
                    files_processed / self.files.len() as f64,
                    &format!(" {files_processed} / {}", self.files.len()),
                ),
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
                    progress_bar.pass();
                }

                installed_fonts
                    .lock()
                    .unwrap()
                    .update_entry(
                        &self.installer_name,
                        InstalledFont {
                            url: self.source.ref_direct_url()?.to_owned(),
                            dir: collapse_home(target_dir),
                            files: self.files.clone(),
                        },
                    )
                    .cleanup(args, &self.installer_name, old_files.as_ref())
                    .map_err(|()| "Failed to cleanup")?;
            }
            true => {
                if !verbose {
                    progress_bar.fail();
                }
                println!("\nErrors were encountered while installing {}", self.name)
            }
        }

        Ok(())
    }

    /// Returns the installer names of all available installers matched
    /// by any of the provided filter patterns
    pub fn filter_installers(filters: &[String]) -> Result<Vec<String>, String> {
        let installers_dir = installers_dir();
        if !Path::new(installers_dir).exists() {
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
            let (pattern, tag) = filter
                .split_once(':')
                .map_or_else(|| (filter.as_ref(), None), |s| (s.0, Some(s.1)));

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

    /// Returns `true` if the font's download URL changes,
    /// or if the installation directory is missing.
    /// Otherwise returns `false`
    #[must_use]
    pub fn has_updates(&self, installed_fonts: &Arc<Mutex<InstalledFonts>>) -> bool {
        installed_fonts
            .lock()
            .unwrap()
            .installed
            .get(&self.installer_name)
            .is_none_or(|installed| {
                self.source.ref_direct_url().unwrap() != installed.url
                    || !fs::exists(installed.get_dir()).unwrap_or_default()
            })
    }
}
