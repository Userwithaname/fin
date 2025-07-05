use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::{env, fs};

#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledFont {
    pub url: String,
    pub dir: String,
}

pub struct InstalledFonts {
    pub installed: BTreeMap<String, InstalledFont>,
    changed: bool,
}

impl InstalledFonts {
    pub fn read() -> Result<Self, String> {
        let file = format!("{}/.config/fin/installed", env::var("HOME").unwrap());
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

    pub fn write(&self) -> Result<(), String> {
        if !self.changed {
            return Ok(());
        }

        let contents = toml::to_string(&self.installed).map_err(|e| {
            eprintln!("Failed to serialize installed fonts to TOML");
            e.to_string()
        })?;
        fs::write(
            format!("{}/.config/fin/installed", env::var("HOME").unwrap()),
            contents,
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Returns the names of all InstalledFonts entries
    pub fn get_names(&self) -> Vec<String> {
        self.installed.clone().into_keys().collect()
    }

    /// Adds a new entry to the installed fonts, or modifies it if it already exists
    pub fn update_entry(&mut self, name: &str, data: InstalledFont) {
        match self.installed.get_mut(name) {
            Some(entry) => *entry = data,
            None => {
                self.installed.insert(name.to_string(), data);
            }
        };
        self.changed = true;
    }

    pub fn remove_entry(&mut self, name: &str) -> Result<(), String> {
        self.installed.remove(name);
        self.changed = true;
        Ok(())
    }
}
