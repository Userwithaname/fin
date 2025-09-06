#[macro_export]
macro_rules! home_dir {
    () => {
        std::env::var("HOME").unwrap()
    };
}

#[macro_export]
macro_rules! config_dir_path {
    () => {
        home_dir!() + "/.config/fin/"
    };
}

#[macro_export]
macro_rules! config_file_path {
    () => {
        config_dir_path!() + "config.toml"
    };
}

#[macro_export]
macro_rules! installers_dir_path {
    () => {
        config_dir_path!() + "installers/"
    };
}

#[macro_export]
macro_rules! installer_path {
    ($name:expr) => {
        installers_dir_path!() + $name
    };
}

#[macro_export]
macro_rules! installed_file_path {
    () => {
        config_dir_path!() + "installed.toml"
    };
}

#[macro_export]
macro_rules! cache_dir {
    () => {
        home_dir!() + "/.cache/fin/"
    };
}

#[macro_export]
macro_rules! page_cache_dir {
    () => {
        cache_dir!() + "page_cache/"
    };
}

#[macro_export]
macro_rules! lock_file_path {
    () => {
        cache_dir!() + "lock_state"
    };
}

#[macro_export]
macro_rules! staging_dir {
    () => {
        cache_dir!() + "staging"
    };
}

#[macro_export]
macro_rules! state_var_name {
    () => {
        "FIN_STATE"
    };
}
