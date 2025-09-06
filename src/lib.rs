use crate::action::Action;
use crate::args::{show_help, Args};
use crate::config::Config;
use crate::font::Font;
use crate::installed::InstalledFonts;
use crate::installer::Installer;

use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const VERSION: &'static str = "0.1.0";

#[macro_use]
pub mod colors;
#[macro_use]
pub mod paths;

pub mod action;
pub mod args;
pub mod bar;
pub mod config;
pub mod wildcards;

mod font;
mod font_page;
mod installed;
mod installer;
mod options;

pub fn run(lock_state: Option<String>) -> Result<(), Box<dyn Error>> {
    let installed_fonts = Arc::new(Mutex::new(InstalledFonts::read()?));

    let interrupt_signal = Arc::new(AtomicBool::new(false));
    ctrlc::set_handler({
        let interrupt_signal = Arc::clone(&interrupt_signal);
        move || {
            interrupt_signal.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    })
    .expect("Error setting Ctrl-C handler");

    let (args, items) = Args::build()?;
    let handle = thread::Builder::new()
        .name("fin".to_string())
        .spawn({
            let lock_state = lock_state.clone();
            let installed_fonts = Arc::clone(&installed_fonts);
            move || action::perform(&args, &items, lock_state.as_ref(), &installed_fonts)
        })
        .unwrap();

    let result = loop {
        thread::sleep(Duration::from_millis(20));
        if handle.is_finished() {
            break handle
                .join()
                .unwrap_or_else(|_| Err("Thread panicked".to_string()))
                .map_err(|e| e.into());
        }
        if interrupt_signal.load(std::sync::atomic::Ordering::Relaxed) {
            drop(handle);
            break Ok(());
        }
    };

    installed_fonts.lock().unwrap().write()?;
    if lock_state.is_none() {
        let _ = fs::remove_file(lock_file_path!());
    }

    result
}

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
    let _ = io::stdout().flush();
    io::stdin().read_line(&mut input).unwrap();

    match input.to_lowercase().as_str() {
        "y\n" | "yes\n" | "yabadabadoo\n" => true,
        "n\n" | "no\n" | "nope\n" => false,
        _ => user_prompt(message, args),
    }
}
