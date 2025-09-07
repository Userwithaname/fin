use crate::args::Args;
use crate::bar;
use crate::home_dir;
use crate::installed_file_path;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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
    pub fn update_entry(&mut self, name: &str, data: InstalledFont) {
        match self.installed.get_mut(name) {
            Some(entry) => *entry = data,
            None => _ = self.installed.insert(name.to_string(), data),
        }
        self.changed = true;
    }

    /// Removes an entry from the installed fonts
    pub fn remove_entry(&mut self, name: &str) -> Result<(), String> {
        self.installed.remove(name);
        self.changed = true;
        Ok(())
    }

    /// Removes the font from disk if it exists
    /// and returns the removed directory
    ///
    /// Returns:
    /// - `Ok(Some(dir))`: upon successful removal
    /// - `Ok(None)`: if the font is not installed
    /// - `Err(…)`: if errors were encountered
    pub fn uninstall(
        &mut self,
        font: &str,
        args: &Args,
        print_name: bool,
    ) -> Result<Option<String>, String> {
        let verbose = args.options.verbose || args.config.verbose_files;
        if let Some(installed_font) = self.installed.get(font) {
            let dir = installed_font.dir.clone();

            let mut dir_iter = dir.split('/');
            dir_iter.next_back();
            let dir_name = dir_iter.next_back().unwrap_or("(unknown)");

            match verbose {
                true => {
                    if print_name {
                        print!("Removing {dir_name}: ");
                    } else {
                        print!("Removing: ");
                    }
                }
                false => {
                    if print_name {
                        println!("\n{dir_name}: ");
                    }
                    print!("… Removing:    ");
                }
            }
            let _ = stdout().flush();

            if !Path::new(&dir).exists() {
                match verbose {
                    true => println!("{} Removing:    {}", orange!("✓"), orange!("Not found")),
                    false => println_orange!("Not found"),
                }
                return self.remove_entry(font).map(|()| Some(dir));
            }

            if verbose {
                println!();
            }

            let result = match args.options.force {
                false => Self::remove_files(installed_font, &dir, dir_name, verbose),
                true => Self::remove_dir_all(&dir, dir_name),
            };

            if result.is_ok() {
                self.remove_entry(font).map(|()| Some(dir))
            } else {
                println!("Errors were encountered while removing: {dir_name}");
                Err("Failed to remove font".to_string())
            }
        } else {
            Ok(None)
        }
    }

    fn remove_files(
        installed_font: &InstalledFont,
        dir: &str,
        dir_name: &str,
        verbose: bool,
    ) -> Result<(), ()> {
        let mut errors = false;

        let update_progress_bar = |status_symbol: &str, progress: f64| {
            bar::show_progress(
                &format!("{} Removing:   ", status_symbol),
                progress / installed_font.files.len() as f64,
                &format!(" {progress} / {}", installed_font.files.len()),
            );
        };

        let mut directories: BTreeSet<String> = [String::new()].into();
        let mut progress = 0.0;
        let mut messages = String::new();
        installed_font.files.iter().for_each(|file| {
            if verbose {
                print!("   {file} ... ");
                let _ = stdout().flush();
            } else {
                progress += 1.0;
                update_progress_bar("…", progress);
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
                        false => messages += &format!("{file}: {}", format_red!("{e}")),
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
                        false => messages += &format!("../{dir_name}/{subdir}: {e}\n"),
                    },
                }
            } else {
                match verbose {
                    true => println_orange!("Not removed: Directory not empty"),
                    false => {
                        messages += &format!(
                            "../{dir_name}/{subdir}: {}\n",
                            format_orange!("Not removed: Directory not empty")
                        )
                    }
                }
            }
        });

        match errors {
            false => {
                if !verbose {
                    update_progress_bar(&green!("✓"), progress);
                    println!();
                }
                print!("{messages}");
                Ok(())
            }
            true => {
                if !verbose {
                    update_progress_bar(&red!("×"), progress);
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
