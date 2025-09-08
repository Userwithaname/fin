use std::fs;

use crate::Config;

pub struct ConfigAction;

impl ConfigAction {
    pub fn show_help() -> String {
        let help = "\
Usage:
    fin config [item]

Items:
    show                  Show the current configuration
    default               Write the default configuration
    delete                Delete the configuration file
    help                  Show this help message
"
        .to_string();

        print!("{help}");

        help
    }

    pub fn run(items: &[String]) -> Result<(), String> {
        if items.is_empty() {
            Self::show_help();
            return Ok(());
        }

        match items[0].as_str() {
            "show" => {
                let target = config_file_path!();
                println!("{}", fs::read_to_string(&target).unwrap_or_default().trim());
            }
            "default" => {
                Config::write_default_config()?;
                println!(
                    "Created a new configuration file on disk:\n{}",
                    config_file_path!()
                );
            }
            "delete" => {
                let target = config_file_path!();
                if fs::exists(&target).unwrap_or_default() {
                    fs::remove_file(&target).map_err(|e| e.to_string())?;
                    println!("Deleted the configuration file:\n{target}");
                } else {
                    println!("The configuration file does not exist");
                }
            }
            "help" => {
                Self::show_help();
            }
            item => {
                Self::show_help();
                println!("\nUnrecognized item: '{item}'");
            }
        }
        Ok(())
    }
}
