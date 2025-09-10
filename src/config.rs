use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub install_dir: String,
    pub cache_timeout: u64,
    pub verbose_mode: bool,
    pub verbose_files: bool,
    pub verbose_list: bool,
    pub verbose_urls: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            install_dir: "~/.local/share/fonts".to_string(),
            cache_timeout: 90,
            verbose_mode: false,
            verbose_files: false,
            verbose_list: false,
            verbose_urls: false,
        }
    }
}

#[macro_export]
macro_rules! default_config {
    // Remember to update README.md
    () => {
        r#"# Location where new fonts will be installed
# Override:  --install-dir=[path]
install_dir = "~/.local/share/fonts"

# How long (in minutes) until cache is considered outdated
# Override:  --cache-timeout=[time]
# Related:   --refresh, --no-refresh
cache_timeout = 90

# Show verbose output by default
# Enable:   --verbose
# Disable:  --no-verbose
verbose_mode = false

# Show verbose output when adding or removing files
# Enable:   --verbose-files,    --verbose
# Disable:  --no-verbose-files, --no-verbose
verbose_files = false

# Show installed paths when running the list command
# Enable:   --verbose-list,    --verbose
# Disable:  --no-verbose-list, --no-verbose
verbose_list = false

# Show URLs in the output
# Enable:   --verbose-urls,    --verbose
# Disable:  --no-verbose-urls, --no-verbose
verbose_urls = false
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
        fs::create_dir_all(config_dir!()).map_err(|e| e.to_string())?;

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
