use crate::args::Args;
use crate::bar;
use crate::home_dir;
use crate::installed_file_path;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::{stdout, Write};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledFont {
    // TODO: Allow locking fonts to a particular tag
    // pub lock: Option<String>,
    pub url: String,
    pub dir: String,
    pub files: Vec<String>,
}

pub struct InstalledFonts {
    pub installed: BTreeMap<String, InstalledFont>,
    changed: bool,
}

impl InstalledFonts {
    /// Reads from `~/.config/fin/installed` and builds an
    /// instance of `InstalledFonts` from it
    pub fn read() -> Result<Self, String> {
        let file = installed_file_path!();

        if !Path::new(&file).exists() {
            return Ok(Self {
                installed: [].into(),
                changed: false,
            });
        }

        let contents = fs::read_to_string(file).map_err(|e| e.to_string())?;

        Ok(Self {
            installed: toml::from_str(&contents).map_err(|e| e.to_string())?,
            changed: false,
        })
    }

    /// Writes `InstalledFonts` to disk in TOML format,
    /// if there are any changes.
    pub fn write(&self) -> Result<(), String> {
        if !self.changed {
            return Ok(());
        }

        let contents = toml::to_string(&self.installed)
            .map_err(|e| {
                eprintln!("Failed to serialize installed fonts to TOML");
                e.to_string()
            })?
            .replace("[\"", "[\n\t\"")
            .replace("\", \"", "\",\n\t\"")
            .replace("\"]", "\"\n]");

        fs::write(installed_file_path!(), contents).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Returns the names of all installed fonts
    #[must_use]
    pub fn get_names(&self) -> Vec<String> {
        self.installed.clone().into_keys().collect()
    }

    /// Adds a new entry to the installed fonts
    /// or modifies it if it already exists
    pub fn update_entry(&mut self, name: &str, data: InstalledFont) -> &mut Self {
        match self.installed.get_mut(name) {
            Some(entry) => *entry = data,
            None => _ = self.installed.insert(name.to_string(), data),
        }
        self.changed = true;
        self
    }

    /// Removes an entry from the installed fonts
    pub fn remove_entry(&mut self, name: &str) {
        self.installed.remove(name);
        self.changed = true;
    }

    pub fn cleanup(
        &self,
        args: &Args,
        font: &str,
        old_files: &Option<Vec<String>>,
    ) -> Result<(), ()> {
        if let (Some(installed_font), Some(old_files)) = (self.installed.get(font), old_files) {
            let stray_files: Vec<String> = old_files
                .iter()
                .filter_map(|file| match installed_font.files.contains(file) {
                    true => None,
                    false => Some(file.to_string()),
                })
                .collect();

            if stray_files.is_empty() {
                return Ok(());
            }

            let mut dir_iter = installed_font.dir.split('/');
            dir_iter.next_back();
            let dir_name = dir_iter.next_back().unwrap_or("(unknown)");

            let verbose = args.options.verbose | args.config.verbose_files;
            if verbose {
                println!("Cleaning up:");
            }

            Self::remove_files(
                &stray_files,
                &installed_font.dir,
                dir_name,
                "Cleaning up:",
                false,
                verbose,
            )?;
        }

        Ok(())
    }

    /// Removes the font from disk if it exists
    /// and returns the removed directory
    ///
    /// Returns:
    /// - `Ok(Some(dir))`: upon successful removal
    /// - `Ok(None)`: if the font is not installed
    /// - `Err(…)`: if errors were encountered
    pub fn uninstall(&mut self, args: &Args, font: &str) -> Result<Option<String>, String> {
        let verbose = args.options.verbose || args.config.verbose_files;
        if let Some(installed_font) = self.installed.get(font) {
            let dir = installed_font.dir.clone();

            let mut dir_iter = dir.split('/');
            dir_iter.next_back();
            let dir_name = dir_iter.next_back().unwrap_or("(unknown)");

            print!("\nRemoving {dir_name}: ");
            if !verbose {
                print!("\n… Removing:    ");
            }
            let _ = stdout().flush();

            if !Path::new(&dir).exists() {
                match verbose {
                    true => println_orange!("Directory not found"),
                    false => println!(
                        "\r{} Removing:    {}",
                        orange!("✓"),
                        orange!("Directory not found")
                    ),
                }
                self.remove_entry(font);
                return Ok(Some(dir));
            }

            if verbose {
                println!();
            }

            let result = match args.options.force {
                false => Self::remove_files(
                    &installed_font.files,
                    &dir,
                    dir_name,
                    "Removing:   ",
                    false,
                    verbose,
                ),
                true => Self::remove_dir_all(&dir, dir_name),
            };

            if result.is_ok() {
                self.remove_entry(font);
                Ok(Some(dir))
            } else {
                println!("Errors were encountered while removing: {dir_name}");
                Err("Failed to remove font".to_string())
            }
        } else {
            Ok(None)
        }
    }

    fn remove_files(
        files: &[String],
        dir: &str,
        dir_name: &str,
        output_prefix: &str,
        warn_not_empty: bool,
        verbose: bool,
    ) -> Result<(), ()> {
        let mut errors = false;

        let update_progress_bar = |status_symbol: &str, files_processed: f64| {
            bar::show_progress(
                &format!("{status_symbol} {}", output_prefix),
                files_processed / files.len() as f64,
                &format!(" {files_processed} / {}", files.len()),
            );
        };

        let mut directories: BTreeSet<String> = [String::new()].into();
        let mut files_processed = 0.0;
        let mut messages = String::new();
        files.iter().for_each(|file| {
            if verbose {
                print!("   {file} ... ");
                let _ = stdout().flush();
            } else {
                files_processed += 1.0;
                update_progress_bar("…", files_processed);
            }

            let file_path = format!("{dir}/{file}");
            let file_path = Path::new(&file_path);
            if !file_path.exists() {
                if verbose {
                    println_orange!("Missing");
                }
                return;
            }

            match fs::remove_file(file_path) {
                Ok(()) => {
                    if verbose {
                        println_green!("Removed")
                    }
                }
                Err(e) => {
                    errors = true;
                    match verbose {
                        true => println_red!("{e}"),
                        false => {
                            let _ = write!(messages, "{file}: {}", format_red!("{e}"));
                        }
                    }
                }
            }

            let mut dirs: Vec<String> = Vec::new();
            let file_path_split: Box<[&str]> = file.split('/').collect();
            for i in 0..file_path_split.len() - 1 {
                if i > 0 {
                    dirs.push(dirs[i - 1].clone() + "/" + file_path_split[i]);
                } else {
                    dirs.push(file_path_split[0].to_string());
                }
                directories.replace(dirs[i].clone());
            }
        });

        directories.iter().rev().for_each(|subdir| {
            if verbose {
                print!("   ../{dir_name}/{subdir} ... ");
                let _ = stdout().flush();
            }

            let target = dir.to_owned() + subdir;
            if fs::read_dir(&target).is_ok_and(|remaining| remaining.count() == 0) {
                match fs::remove_dir(&target) {
                    Ok(()) => {
                        if verbose {
                            println_green!("Removed")
                        }
                    }
                    Err(e) => match verbose {
                        true => println_red!("{e}"),
                        false => {
                            let _ = writeln!(messages, "../{dir_name}/{subdir}: {e}");
                        }
                    },
                }
            } else if warn_not_empty {
                match verbose {
                    true => println_orange!("Not removed: Directory not empty"),
                    false => {
                        let _ = writeln!(
                            messages,
                            "../{dir_name}/{subdir}: {}",
                            format_orange!("Not removed: Directory not empty")
                        );
                    }
                }
            }
        });

        match errors {
            false => {
                if !verbose {
                    update_progress_bar(&green!("✓"), files_processed);
                    println!();
                }
                print!("{messages}");
                Ok(())
            }
            true => {
                if !verbose {
                    update_progress_bar(&red!("×"), files_processed);
                    println!();
                }
                print!("{messages}");
                Err(())
            }
        }
    }

    fn remove_dir_all(dir: &str, dir_name: &str) -> Result<(), ()> {
        let mut errors = false;

        print!("   ../{dir_name} ... ");
        let _ = stdout().flush();

        match fs::remove_dir_all(dir).map_err(|e| e.to_string()) {
            Ok(()) => println_green!("Removed"),
            Err(e) => {
                errors = true;
                println_red!("{e}");
            }
        }

        match errors {
            false => Ok(()),
            true => Err(()),
        }
    }
}
