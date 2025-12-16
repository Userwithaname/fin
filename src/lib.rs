use crate::args::Args;
use crate::config::Config;
use crate::font::Font;
use crate::installed::InstalledFonts;
use crate::installer::Installer;
use crate::paths::lock_file_path;

use std::fs;
use std::io::{stdin, stdout, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

#[macro_use]
pub mod colors;

pub mod action;
pub mod actions;
pub mod args;
pub mod bar;
pub mod checksum;
pub mod config;
pub mod file_action;
pub mod font_page;
pub mod installer;
pub mod options;
pub mod paths;
pub mod source;
pub mod wildcards;

mod font;
mod installed;

pub fn run(lock_state: Option<String>) -> Result<(), String> {
    let (interrupt, result) = mpsc::channel::<Result<(), String>>();
    let installed_fonts = Arc::new(Mutex::new(InstalledFonts::read()?));

    thread::Builder::new()
        .name("fin".to_string())
        .spawn({
            let interrupt = interrupt.clone();
            let (args, items) = Args::build()?;
            let lock_state = lock_state.clone();
            let installed_fonts = Arc::clone(&installed_fonts);
            move || {
                let result = action::perform(&args, &items, lock_state.as_ref(), &installed_fonts);
                interrupt.send(result).unwrap();
            }
        })
        .unwrap();

    ctrlc::set_handler(move || {
        interrupt.send(Err("Interrupted by user".into())).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    let result = result.recv().unwrap();

    if lock_state.is_none() {
        let _ = fs::remove_file(lock_file_path());
    }

    installed_fonts.lock().unwrap().write()?;
    result
}

/// Prompts the user to approve or deny, and waits for an answer.
/// Returns `true` if 'yes', or `false` if 'no'
#[inline]
#[must_use]
pub fn user_prompt(message: &str, args: &Args) -> bool {
    print!("{message} [y/n]: ");

    match args.options.answer {
        Some(false) => {
            println!("no");
            return false;
        }
        Some(true) => {
            println!("yes");
            return true;
        }
        None => {}
    }

    let mut input = String::new();
    let _ = stdout().flush();
    stdin().read_line(&mut input).unwrap();

    match input.to_lowercase().as_str() {
        "y\n" | "yes\n" | "yabadabadoo\n" => true,
        "n\n" | "no\n" | "nope\n" => false,
        _ => user_prompt(message, args),
    }
}

/// Converts bytes into more useful units and returns a formatted string
#[must_use]
pub fn format_size(mut num_bytes: f64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut unit_index = 0;
    while num_bytes > 1023.0 && unit_index < UNITS.len() {
        num_bytes /= 1024.0;
        unit_index += 1;
    }
    format!("{num_bytes:.1} {}", UNITS[unit_index])
}
