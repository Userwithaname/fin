use crate::Config;
use crate::Font;
use crate::InstalledFonts;

use std::env;

pub enum Action {
    Install,
    Update,
    Remove,
    List,
    Help,
}

pub struct Options {
    pub reinstall: bool,
    pub refresh: bool,
    pub verbose: bool,
    pub answer: Option<bool>,
}

pub struct Args {
    pub action: Action,
    pub fonts: Box<[Font]>,
    pub options: Options,
    pub config: Config,
}

impl Args {
    /// Loads the user-specified actions and arguments
    pub fn build(installed_fonts: &mut InstalledFonts) -> Result<Self, String> {
        let mut args = env::args();
        args.next();
        let action = match args.next() {
            Some(a) => match a.as_str() {
                "install" | "get" => Action::Install,
                "update" | "upgrade" | "up" => Action::Update,
                "remove" | "uninstall" | "rm" => Action::Remove,
                "list" | "ls" => Action::List,
                "help" | "h" => Action::Help,
                _ => {
                    show_help();
                    println!();
                    return Err(format!("Unrecognized action: {a}"));
                }
            },
            None => Action::Help,
        };

        let mut fonts = Vec::new();
        let mut flags = Vec::new();
        for item in args {
            if item.chars().nth(0).unwrap() == '-' {
                flags.push(item);
            } else {
                fonts.push(item);
            }
        }

        let mut options = Options {
            reinstall: false,
            refresh: false,
            verbose: false,
            answer: None,
        };
        let mut config = Config::load()?;

        for flag in flags {
            let opt_val = &mut flag.split('=');
            let (opt, val) = (opt_val.next().unwrap(), opt_val.next());
            match opt {
                "--install-dir" => config.install_dir = val.unwrap().to_string(),
                "--cache-timeout" => {
                    config.cache_timeout = val.unwrap().parse::<u64>().map_err(|e| e.to_string())?
                }

                "--reinstall" => options.reinstall = true,
                "--refresh" => options.refresh = true,
                "--verbose" => options.verbose = true,
                "--yes" => options.answer = Some(true),
                "--no" => options.answer = Some(false),

                opt => {
                    let mut opts = opt.chars();
                    if opts.next().unwrap() == '-' {
                        for o in opts {
                            match o {
                                'i' => options.reinstall = true,
                                'r' => options.refresh = true,
                                'v' => options.verbose = true,
                                'y' => options.answer = Some(true),
                                'n' => options.answer = Some(false),
                                '-' => {
                                    show_help();
                                    println!();
                                    return Err(format!("Unknown argument: {opt}"));
                                }
                                o => {
                                    show_help();
                                    println!();
                                    return Err(format!("Unknown argument: -{o}"));
                                }
                            }
                        }
                    } else {
                        show_help();
                        println!();
                        return Err(format!("Unknown argument: {opt}"));
                    }
                }
            }
        }

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

Actions:
    install               Install new fonts
    update                Update installed fonts
    remove                Remove installed fonts
    list                  List installed or available fonts
    help                  Show this help message

Arguments:
    --install-dir=[path]  Sets the installation directory
    --reinstall  -i       Skip version checks and reinstall
    --refresh    -r       Ignore cache and fetch new data
    --verbose    -v       Show more detailed output
    --yes        -y       Automatically accept prompts
"
    );
}
