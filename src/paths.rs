use crate::wildcards::wildcard_substring;
use std::env;
use std::sync::OnceLock;

pub fn expand_tilde(path: &mut String) {
    if wildcard_substring(path, "^~/", b"").is_some() {
        path.replace_range(..1, home_dir());
    }
}

fn home_dir() -> &'static String {
    static HOME_DIR: OnceLock<String> = OnceLock::new();
    HOME_DIR.get_or_init(|| {
        env::home_dir()
            .expect("Home directory not found")
            .to_str()
            .unwrap()
            .to_owned()
            + "/"
    })
}

pub fn config_dir() -> &'static String {
    static CONFIG_DIR: OnceLock<String> = OnceLock::new();
    CONFIG_DIR.get_or_init(|| {
        env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| [home_dir(), ".config"].concat()) + "/fin/"
    })
}

pub fn cache_dir() -> &'static String {
    static CACHE_DIR: OnceLock<String> = OnceLock::new();
    CACHE_DIR.get_or_init(|| {
        env::var("XDG_CACHE_HOME").unwrap_or_else(|_| [home_dir(), ".cache"].concat()) + "/fin/"
    })
}

pub fn installers_dir() -> &'static String {
    static INSTALLERS_DIR: OnceLock<String> = OnceLock::new();
    INSTALLERS_DIR.get_or_init(|| [config_dir(), "installers/"].concat())
}

pub fn page_cache_dir() -> &'static String {
    static PAGE_CACHE_DIR: OnceLock<String> = OnceLock::new();
    PAGE_CACHE_DIR.get_or_init(|| [cache_dir(), "page_cache/"].concat())
}

pub fn staging_dir() -> &'static String {
    static STAGING_DIR: OnceLock<String> = OnceLock::new();
    STAGING_DIR.get_or_init(|| [cache_dir(), "staging/"].concat())
}

pub fn installed_file_path() -> &'static String {
    static INSTALLED_FILE_PATH: OnceLock<String> = OnceLock::new();
    INSTALLED_FILE_PATH.get_or_init(|| [config_dir(), "installed.toml"].concat())
}

pub fn config_file_path() -> &'static String {
    static CONFIG_FILE_PATH: OnceLock<String> = OnceLock::new();
    CONFIG_FILE_PATH.get_or_init(|| [config_dir(), "config.toml"].concat())
}

pub fn lock_file_path() -> &'static String {
    static LOCK_FILE_PATH: OnceLock<String> = OnceLock::new();
    LOCK_FILE_PATH.get_or_init(|| [config_dir(), "lock_state"].concat())
}
