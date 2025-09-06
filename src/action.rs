use crate::Args;
use crate::Config;
use crate::Font;
use crate::InstalledFonts;
use crate::Installer;

use crate::{show_help, user_prompt};

use std::fs;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub enum Action {
    Install,
    Reinstall,
    Update,
    Remove,
    List,
    Init,
    Clean,
    Help,
}

impl Action {
    #[must_use]
    pub const fn help_actions() -> &'static str {
        // Remember to update README.md
        "\
Actions:
    install               Install new fonts
    reinstall             Reinstall fonts
    update                Update installed fonts
    remove                Remove installed fonts
    clean                 Remove temporary cache files
    list                  List installed or available fonts
    init                  Create a new configuration file
    help                  Show this help message
"
    }

    pub fn parse(action: Option<String>) -> Result<Self, String> {
        Ok(match action {
            Some(a) => match a.as_str() {
                "install" | "get" => Action::Install,
                "reinstall" => Action::Reinstall,
                "update" | "upgrade" | "up" => Action::Update,
                "remove" | "uninstall" | "rm" => Action::Remove,
                "clean" => Action::Clean,
                "list" | "ls" => Action::List,
                "init" | "land" => Action::Init,
                "help" | "h" => Action::Help,
                _ => {
                    show_help();
                    println!();
                    return Err(format!("Unrecognized action: {a}"));
                }
            },
            None => Action::Help,
        })
    }
}

pub fn perform(
    args: &Args,
    items: &[String],
    lock_state: Option<&String>,
    installed_fonts: &Arc<Mutex<InstalledFonts>>,
) -> Result<(), String> {
    let init_fonts = |require_valid_config: bool,
                      lock_action: Option<&str>,
                      no_fonts_message: &str|
     -> Option<Box<[Font]>> {
        if require_valid_config {
            args.config.panic_if_invalid();
        }

        if let Some(lock_action) = lock_action {
            if let Some(lock_state) = lock_state {
                println!("Install state is locked; refusing to continue");
                println!("It looks like another instance is currently {lock_state} something");
                println!("If this is not the case, you may run `fin clean state --force` to manually unlock it");
                panic!("Lock file exists; refusing to continue");
            }

            let _ = fs::create_dir_all(cache_dir!());
            let _ = fs::write(lock_file_path!(), lock_action);
        }

        let fonts: Box<[Font]> =
            Font::get_actionable_fonts(&Arc::new(args.clone()), items, installed_fonts)
                .inspect_err(|e| panic!("{e}"))
                .unwrap()
                .into();

        if fonts.is_empty() {
            println!("{no_fonts_message}");
            None
        } else {
            Some(fonts)
        }
    };

    match args.action {
        Action::Install => 'install: {
            let mut fonts = match init_fonts(true, Some("installing"), "Nothing new to install") {
                Some(fonts) => fonts,
                None => return Ok(()),
            };

            println!("Installing: ");
            Args::list_fonts_green(&fonts);

            // TODO: Inform the user of the total download size
            if !user_prompt("Proceed?", args) {
                break 'install;
            }

            install_fonts(args, &mut fonts, installed_fonts)?;
        }
        Action::Reinstall => 'reinstall: {
            let mut fonts = match init_fonts(true, Some("reinstalling"), "Nothing to reinstall") {
                Some(fonts) => fonts,
                None => return Ok(()),
            };

            println!("Installing: ");
            Args::list_fonts_green(&fonts);

            if !user_prompt("Proceed?", args) {
                break 'reinstall;
            }

            install_fonts(args, &mut fonts, installed_fonts)?;
        }
        Action::Update => 'update: {
            let mut fonts = match init_fonts(true, Some("updating"), "No updates found") {
                Some(fonts) => fonts,
                None => return Ok(()),
            };

            println!("Updating: ");
            Args::list_fonts_green(&fonts);

            if !user_prompt("Proceed?", args) {
                break 'update;
            }

            install_fonts(args, &mut fonts, installed_fonts)?;
        }
        Action::Remove => 'remove: {
            let fonts = match init_fonts(true, Some("removing"), "Nothing to remove") {
                Some(fonts) => fonts,
                None => return Ok(()),
            };

            println!("Removing: ");
            Args::list_fonts_red(&fonts);

            if !user_prompt("Proceed?", args) {
                break 'remove;
            }

            remove_fonts(args, &fonts, installed_fonts)?;
        }
        Action::List => {
            let fonts = match init_fonts(false, None, "Nothing to list") {
                Some(fonts) => fonts,
                None => return Ok(()),
            };

            fonts.iter().for_each(|font| {
                match installed_fonts.lock().unwrap().installed.get(&font.name) {
                    Some(installed) => {
                        if Font::has_installer(&font.name) {
                            match args.options.verbose || args.config.verbose_list {
                                true => {
                                    println!("{}\n ↪ {}", format_green!("{font}"), installed.dir);
                                }
                                false => println_green!("{font}"),
                            }
                        } else {
                            match args.options.verbose {
                                true => println!(
                                    "{}\n ↪ {}",
                                    format_orange!("{font} (missing installer)"),
                                    installed.dir
                                ),
                                false => println_orange!("{font} (missing installer)"),
                            }
                        }
                    }
                    None => {
                        println!("{font}");
                    }
                }
            });
        }
        Action::Clean => {
            if lock_state.is_some() && !args.options.force {
                println!("Cleaning the cache while another instance is running is not recommended");
                println!("Note: try passing `--force` to clean it anyway");
                return Err(
                    "Attempted to alter cache while another instance was running".to_string(),
                );
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
                            println!("Removed the cache directory: {}", target);
                        }
                    }
                    "pages" => {
                        let target = page_cache_dir!();
                        if fs::exists(&target).unwrap_or(true) {
                            fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
                            println!("Removed the page cache directory: {}", target);
                        }
                    }
                    "staging" => {
                        let target = staging_dir!();
                        if fs::exists(&target).unwrap_or(true) {
                            fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
                            println!("Removed the staging directory: {}", target);
                        }
                    }
                    "state" => {
                        let target = lock_file_path!();
                        if fs::exists(&target).unwrap_or(true) {
                            fs::remove_file(&target).map_err(|e| e.to_string())?;
                            println!("Removed the lock file: {}", target);
                        }
                    }
                    _ => {
                        println!("Cannot clean {item}");
                        println!("Supported items: [all/pages/staging/state]");
                    }
                }
            }
        }
        Action::Init => {
            Config::write_default_config()?;
            println!(
                "Created a new configuration file on disk:\n{}",
                config_file_path!()
            );
        }
        Action::Help => {
            show_help();
        }
    }

    if lock_state.is_none() {
        let _ = fs::remove_file(lock_file_path!());
    }

    Ok(())
}

// IDEA: Parallel downloads, only install after all downloads are done
fn install_fonts(
    args: &Args,
    fonts: &mut Box<[Font]>,
    installed_fonts: &Arc<Mutex<InstalledFonts>>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    fonts.iter_mut().for_each(|font| {
        if let Some(installer) = &mut font.installer {
            match download_and_install(args, installer, installed_fonts) {
                Ok(()) => (),
                Err(e) => {
                    println!("Failed to install {}:\n{}", installer.name, red!(&e));
                    errors.push(format!("{font}: {}", red!(&e)));
                }
            }
        } else {
            println!("Failed to install {font}");
            println_red!("Installer for '{font}' has not been loaded");
            errors.push(format!(
                "{}: {}",
                font,
                red!("Installer has not been loaded")
            ));
        }
    });

    if errors.is_empty() {
        Ok(())
    } else {
        println!("\nFailed:");
        errors.iter().for_each(|e| println!("   {e}"));
        Err("One or more fonts failed to install".to_string())
    }
}

fn download_and_install(
    args: &Args,
    installer: &mut Installer,
    installed_fonts: &Arc<Mutex<InstalledFonts>>,
) -> Result<(), String> {
    installer
        .download_font()?
        .prepare_install()?
        .finalize_install(args, installed_fonts)
}

fn remove_fonts(
    args: &Args,
    fonts: &[Font],
    installed_fonts: &Arc<Mutex<InstalledFonts>>,
) -> Result<(), String> {
    fonts.iter().try_for_each(|font| {
        println!();
        installed_fonts
            .lock()
            .unwrap()
            .uninstall(&font.name, args)
            .map(|_| ())
    })?;
    Ok(())
}
