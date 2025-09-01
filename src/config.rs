use crate::config_dir_path;
use crate::config_file_path;
use crate::home_dir;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub install_dir: String,
    pub cache_timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            install_dir: "~/.fonts".to_string(),
            cache_timeout: 90,
        }
    }
}

#[macro_export]
macro_rules! default_config {
    // Remember to update README.md
    () => {
        r#"# Default location for installing new fonts:
install_dir = "~/.fonts"

# How long (in minutes) until cache is considered outdated:
cache_timeout = 90
"#
    };
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let config_file = config_file_path!();
        if !Path::new(&config_file).exists() {
            return Self::write_default_config();
        }

        let config: Self = toml::from_str(&fs::read_to_string(config_file).map_err(|err| {
            eprintln!("Config file could not be read from disk.");
            err.to_string()
        })?)
        .map_err(|err| {
            eprintln!("Problems parsing the config file.");
            err.to_string()
        })?;
        Ok(config)
    }

    pub fn write_default_config() -> Result<Self, String> {
        fs::create_dir_all(config_dir_path!()).map_err(|e| e.to_string())?;
        fs::write(config_file_path!(), default_config!()).map_err(|e| e.to_string())?;
        Ok(Self::default())
    }

    pub fn panic_if_invalid(&self) {
        assert!(
            self.install_dir.trim().split('/').count() > 2,
            "The specified installation directory is invalid"
        );
    }
}
