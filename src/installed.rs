use crate::args::Args;
use crate::home_dir;
use crate::installed_file_path;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
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
    pub fn uninstall(&mut self, font: &str, args: &Args) -> Result<Option<String>, String> {
        if let Some(installed_font) = self.installed.get(font) {
            let dir = installed_font.dir.clone();

            let mut dir_iter = dir.split('/');
            dir_iter.next_back();
            let dir_name = dir_iter.next_back().unwrap_or("(unknown)");

            println!("Removing {dir_name}: ");

            if !Path::new(&dir).exists() {
                println_orange!("Not found");
                return self.remove_entry(font).map(|()| Some(dir));
            }

            let result = match args.options.force {
                false => Self::remove_files(installed_font, &dir, dir_name),
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

    fn remove_files(installed_font: &InstalledFont, dir: &str, dir_name: &str) -> Result<(), ()> {
        let mut errors = false;

        let mut directories: BTreeSet<String> = [String::new()].into();
        installed_font.files.iter().for_each(|file| {
            print!("   {file} ... ");
            let _ = io::stdout().flush();

            let file_path = format!("{dir}/{file}");
            let file_path = Path::new(&file_path);
            if !file_path.exists() {
                println_orange!("Missing");
                return;
            }

            match fs::remove_file(file_path) {
                Ok(()) => println_green!("Removed"),
                Err(e) => {
                    errors = true;
                    println_red!("{e}");
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
            print!("   ../{dir_name}/{subdir} ... ");
            let _ = io::stdout().flush();

            let target = dir.to_owned() + subdir;
            if fs::read_dir(&target).is_ok_and(|remaining| remaining.count() == 0) {
                match fs::remove_dir(&target) {
                    Ok(()) => println_green!("Removed"),
                    Err(e) => println_red!("{e}"),
                }
            } else {
                println_orange!("Not removed: Directory not empty");
            }
        });

        match errors {
            false => Ok(()),
            true => Err(()),
        }
    }

    fn remove_dir_all(dir: &str, dir_name: &str) -> Result<(), ()> {
        let mut errors = false;

        print!("   ../{dir_name} ... ");
        let _ = io::stdout().flush();

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
