use crate::actions::{
    clean::CleanAction, config::ConfigAction, help::HelpAction, install::InstallAction,
    list::ListAction, reinstall::ReinstallAction, remove::RemoveAction, update::UpdateAction,
    version::VersionAction,
};
use crate::paths::{cache_dir, lock_file_path};

use crate::Args;
use crate::Font;
use crate::InstalledFonts;

use std::fs;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum Action {
    Install,
    Reinstall,
    Update,
    Remove,
    List,
    Clean,
    Config,
    Version,
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
    list                  List installed or available fonts
    clean                 Remove temporary cache files
    config                Manage the configuration file
    version               Show the current version number
    help                  Show help for any action
"
    }

    pub fn parse(action: Option<&String>) -> Result<Self, String> {
        Ok(match action {
            Some(a) => match a.as_str() {
                "install" | "get" => Action::Install,
                "reinstall" => Action::Reinstall,
                "update" | "upgrade" | "up" => Action::Update,
                "remove" | "uninstall" | "rm" => Action::Remove,
                "list" | "ls" => Action::List,
                "clean" | "clear" => Action::Clean,
                "config" | "cfg" => Action::Config,
                "version" | "ver" | "v" => Action::Version,
                "help" | "h" => Action::Help,
                _ => {
                    HelpAction::run(&Action::Help);
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

            let _ = fs::create_dir_all(cache_dir());
            let _ = fs::write(lock_file_path(), lock_action);
        }

        let fonts: Box<[Font]> =
            Font::get_actionable_fonts(&Arc::new(args.clone()), items, installed_fonts)
                .inspect_err(|e| panic!("{e}"))
                .unwrap()
                .into();

        if fonts.is_empty() {
            if !no_fonts_message.is_empty() {
                println!("{no_fonts_message}");
            }
            None
        } else {
            Some(fonts)
        }
    };

    match args.action {
        Action::Install => {
            let Some(mut fonts) = init_fonts(true, Some("installing"), "Nothing new to install")
            else {
                return Ok(());
            };
            InstallAction::run(args, &mut fonts, installed_fonts)?;
        }
        Action::Reinstall => {
            let Some(mut fonts) = init_fonts(true, Some("reinstalling"), "Nothing to reinstall")
            else {
                return Ok(());
            };
            ReinstallAction::run(args, &mut fonts, installed_fonts)?;
        }
        Action::Update => {
            let Some(mut fonts) = init_fonts(true, Some("updating"), "No updates found") else {
                return Ok(());
            };
            UpdateAction::run(args, &mut fonts, installed_fonts)?;
        }
        Action::Remove => {
            let Some(fonts) = init_fonts(true, Some("removing"), "Nothing to remove") else {
                return Ok(());
            };
            RemoveAction::run(args, &fonts, installed_fonts)?;
        }
        Action::List => {
            let Some(fonts) = init_fonts(false, None, "") else {
                return Ok(());
            };
            ListAction::run(args, &fonts, Arc::clone(installed_fonts));
        }
        Action::Clean => CleanAction::run(args, items, lock_state)?,
        Action::Config => ConfigAction::run(items)?,
        Action::Version => VersionAction::run(),
        Action::Help => {
            let action = Action::parse(items.iter().next())?;
            HelpAction::run(&action);
        }
    }

    Ok(())
}
