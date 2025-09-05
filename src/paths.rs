#![macro_use]

#[macro_export]
macro_rules! home_dir {
    () => {
        std::env::var("HOME").unwrap()
    };
}

#[macro_export]
macro_rules! cache_dir {
    () => {
        home_dir!() + "/.cache/fin/"
    };
}

#[macro_export]
macro_rules! staging_dir {
    () => {
        home_dir!() + "/.cache/fin/staging"
    };
}

#[macro_export]
macro_rules! installers_dir_path {
    () => {
        home_dir!() + "/.config/fin/installers/"
    };
}

#[macro_export]
macro_rules! installer_path {
    ($name:expr) => {
        installers_dir_path!() + $name
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
        home_dir!() + "/.config/fin/config.toml"
    };
}

#[macro_export]
macro_rules! installed_file_path {
    () => {
        home_dir!() + "/.config/fin/installed.toml"
    };
}
