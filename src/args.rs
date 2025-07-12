use crate::options::Options;
use crate::Action;
use crate::Config;
use crate::Font;
use crate::InstalledFonts;

use std::env;

pub struct Args {
    pub action: Action,
    pub fonts: Box<[Font]>,
    pub config: Config,
    pub options: Options,
}

impl Args {
    /// Loads the user-specified actions and arguments
    pub fn build(installed_fonts: &mut InstalledFonts) -> Result<Self, String> {
        let mut args = env::args();
        args.next();

        let action = Action::parse(args.next())?;

        let mut fonts = Vec::new();
        let mut flags = Vec::new();
        for item in args {
            if item.chars().nth(0).unwrap() == '-' {
                flags.push(item);
            } else {
                fonts.push(item);
            }
        }

        let mut config = Config::load()?;
        let options = Options::build(&flags, &mut config)?;

        config.install_dir = config
            .install_dir
            .replace("~/", &format!("{}/", env::var("HOME").unwrap()));

        let mut args = Args {
            action,
            fonts: [].into(),
            options,
            config,
        };

        args.fonts = Font::get_actionable_fonts(&args, &fonts, installed_fonts)
            .map_err(|e| e.to_string())?
            .into();

        Ok(args)
    }
}

pub fn show_help() {
    // Remember to update README.md
    print!(
        "\
Usage:
    fin [action] [items]

{}
{}",
        Action::help_actions(),
        Options::help_options()
    );
}
