use std::fs;

use crate::args::Args;

pub struct CleanAction;

impl CleanAction {
    pub fn show_help() -> String {
        let help = "\
Usage:
    fin clean [item(s)]

Items:
    all                   Remove all cache
    pages                 Remove cached pages
    staging               Remove the staging directory
    state                 Clear the install state lock
    help                  Show this help message
"
        .to_string();

        print!("{help}");

        help
    }

    pub fn run(args: &Args, items: &[String], lock_state: Option<&String>) -> Result<(), String> {
        if lock_state.is_some() && !args.options.force {
            println!("Cleaning the cache while another instance is running is not recommended");
            println!("Note: try passing `--force` to clean it anyway");
            return Err("Attempted to alter cache while another instance was running".to_string());
        }

        let items = match items.is_empty() {
            true => &["all".to_string()],
            false => items,
        };

        for item in items {
            match item.as_str() {
                "all" => {
                    let target = cache_dir!();
                    if fs::exists(&target).unwrap_or(true) {
                        fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
                        println!("Removed the cache directory: {target}");
                    }
                }
                "pages" => {
                    let target = page_cache_dir!();
                    if fs::exists(&target).unwrap_or(true) {
                        fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
                        println!("Removed the page cache directory: {target}");
                    }
                }
                "staging" => {
                    let target = staging_dir!();
                    if fs::exists(&target).unwrap_or(true) {
                        fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
                        println!("Removed the staging directory: {target}");
                    }
                }
                "state" => {
                    let target = lock_file_path!();
                    if fs::exists(&target).unwrap_or(true) {
                        fs::remove_file(&target).map_err(|e| e.to_string())?;
                        println!("Removed the lock file: {target}");
                    }
                }
                "help" => {
                    Self::show_help();
                }
                _ => {
                    Self::show_help();
                    println!("\nCannot clean '{item}'");
                }
            }
        }
        Ok(())
    }
}
