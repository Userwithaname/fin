use crate::actions::clean::CleanAction;
use crate::actions::config::ConfigAction;
use crate::actions::install::InstallAction;
use crate::actions::list::ListAction;
use crate::actions::reinstall::ReinstallAction;
use crate::actions::remove::RemoveAction;
use crate::actions::update::UpdateAction;
use crate::actions::version::VersionAction;

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
            Action::Clean => CleanAction::show_help(),
            Action::Config => ConfigAction::show_help(),
            Action::Help => Self::show_help(),
            Action::List => ListAction::show_help(),
            Action::Install => InstallAction::show_help(),
            Action::Reinstall => ReinstallAction::show_help(),
            Action::Update => UpdateAction::show_help(),
            Action::Remove => RemoveAction::show_help(),
            Action::Version => VersionAction::show_help(),
        }
    }
}
