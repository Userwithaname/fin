use crate::actions::{
    clean::CleanAction, config::ConfigAction, install::InstallAction, list::ListAction,
    reinstall::ReinstallAction, remove::RemoveAction, update::UpdateAction, version::VersionAction,
};

use crate::action::Action;
use crate::options::Options;

pub struct HelpAction;

impl HelpAction {
    pub fn show_help() -> String {
        // Remember to update README.md
        let help = format!(
            "\
Usage:
    fin [action] [items]

{}
{}",
            Action::help_actions(),
            Options::help_options()
        );

        print!("{help}");

        help
    }

    pub fn run(action: &Action) -> String {
        match action {
            Action::Install => InstallAction::show_help(),
            Action::Reinstall => ReinstallAction::show_help(),
            Action::Update => UpdateAction::show_help(),
            Action::Remove => RemoveAction::show_help(),
            Action::List => ListAction::show_help(),
            Action::Clean => CleanAction::show_help(),
            Action::Config => ConfigAction::show_help(),
            Action::Version => VersionAction::show_help(),
            Action::Help => Self::show_help(),
        }
    }
}
