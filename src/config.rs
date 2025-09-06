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
    pub verbose_mode: bool,
    pub verbose_list: bool,
    pub verbose_urls: bool,
    pub verbose_files: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            install_dir: "~/.local/share/fonts".to_string(),
            cache_timeout: 90,
            verbose_mode: false,
            verbose_list: false,
            verbose_files: false,
            verbose_urls: false,
        }
    }
}

#[macro_export]
macro_rules! default_config {
    // Remember to update README.md
    () => {
        r#"# Default location for installing new fonts
install_dir = "~/.local/share/fonts"

# How long (in minutes) until cache is considered outdated
cache_timeout = 90

# Show verbose output by default (pass --no-verbose to negate)
verbose_mode = false

# Show verbose cache-related output
verbose_urls = false

# Show installed paths when running the 'list' command
verbose_list = false

# Show verbose output when adding or removing files
verbose_files = false
"#
    };
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let config_file = config_file_path!();
        if !Path::new(&config_file).exists() {
            return Ok(Self::default());
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

    pub fn write_default_config() -> Result<(), String> {
        fs::create_dir_all(config_dir_path!()).map_err(|e| e.to_string())?;

        let config_file_path = config_file_path!();
        let config_file = Path::new(&config_file_path);
        if config_file.exists() {
            let _ = fs::rename(config_file, Path::new(&(config_file_path!() + "~")));
        }

        fs::write(config_file_path!(), default_config!()).map_err(|e| e.to_string())
    }

    pub fn panic_if_invalid(&self) {
        assert!(
            self.install_dir.trim().split('/').count() > 2,
            "The specified installation directory is invalid"
        );
    }
}
