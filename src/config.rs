use serde::Deserialize;
use std::{env, fs, path::Path};

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    pub install_dir: String,
    pub cache_timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            install_dir: format!("{}/.fonts/", env::var("HOME").unwrap()),
            cache_timeout: 90,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let config_file = format!("{}/.config/fin/config.toml", env::var("HOME").unwrap());
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
}
