use crate::font::Font;
use crate::options::Options;
use crate::Action;
use crate::Config;

use std::env;

#[derive(Clone)]
pub struct Args {
    pub action: Action,
    pub config: Config,
    pub options: Options,
}

impl Args {
    /// Loads the user-specified actions and arguments
    pub fn build() -> Result<(Self, Vec<String>), String> {
        let mut args = env::args();
        args.next();

        let action = Action::parse(args.next())?;

        let mut flags = Vec::new();
        let mut items = Vec::new();
        for item in args {
            if item.chars().nth(0).unwrap() == '-' {
                flags.push(item);
            } else {
                items.push(item);
            }
        }

        let mut config = Config::load()?;
        let options = Options::build(&flags, &mut config)?;

        config.install_dir = config
            .install_dir
            .replace("~/", &format!("{}/", env::var("HOME").unwrap()));

        Ok((
            Args {
                action,
                config,
                options,
            },
            items,
        ))
    }

    pub fn list_fonts_green(fonts: &[Font]) {
        fonts.iter().for_each(|font| println_green!("   {font}"));
        println!();
    }

    pub fn list_fonts_red(fonts: &[Font]) {
        fonts.iter().for_each(|font| println_red!("   {font}"));
        println!();
    }
}

pub fn show_help() -> String {
    // Remember to update README.md
    let help_message = format!(
        "\
Usage:
    fin [action] [items]

{}
{}",
        Action::help_actions(),
        Options::help_options()
    );

    print!("{help_message}");

    help_message
}
