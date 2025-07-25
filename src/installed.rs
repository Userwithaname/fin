use crate::home_dir;
use crate::installed_file_path;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledFont {
    // TODO: Allow locking fonts to a particular tag
    // pub lock: Option<String>,
    pub url: String,
    pub dir: String,
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

        let contents = toml::to_string(&self.installed).map_err(|e| {
            eprintln!("Failed to serialize installed fonts to TOML");
            e.to_string()
        })?;
        fs::write(installed_file_path!(), contents).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Returns the names of all installed fonts
    pub fn get_names(&self) -> Vec<String> {
        self.installed.clone().into_keys().collect()
    }

    /// Adds a new entry to the installed fonts
    /// or modifies it if it already exists
    pub fn update_entry(&mut self, name: &str, data: InstalledFont) {
        match self.installed.get_mut(name) {
            Some(entry) => *entry = data,
            None => _ = self.installed.insert(name.to_string(), data),
        };
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
    pub fn uninstall(&mut self, font: &str) -> Result<Option<String>, String> {
        if let Some(installed_font) = self.installed.get(font) {
            print!("Removing {} ... ", font);

            let mut errors = false;
            let dir = installed_font.dir.clone();

            // TODO: Remember which files were installed, and only remove those
            //       (& remove directory if left empty)
            match fs::remove_dir_all(dir.clone()).map_err(|e| e.to_string()) {
                Ok(_) => println!("\x1b[92mDone\x1b[0m"),
                Err(e) => {
                    errors = true;
                    println!("\x1b[91m{e}\x1b[0m")
                }
            }

            if !errors {
                self.remove_entry(font).map(|_| Some(dir.clone()))
            } else {
                Err("Failed to remove font".to_string())
            }
        } else {
            println!();
            Ok(None)
        }
    }
}
