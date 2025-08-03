use crate::show_help;
use crate::Config;

#[derive(Default)]
pub struct Options {
    pub reinstall: bool,
    pub refresh: bool,
    pub verbose: bool,
    pub answer: Option<bool>,
    pub force: bool,
}

impl Options {
    #[must_use]
    pub const fn help_options() -> &'static str {
        // Remember to update README.md
        "\
Arguments:
    --install-dir=[path]  Set the installation directory
    --reinstall     -i    Skip version checks and reinstall
    --refresh       -r    Ignore cache and fetch new data
    --cache-only    -c    Do not fetch new data if possible
    --verbose       -v    Show more detailed output
    --force         -f    Forcefully perform action (unsafe)
    --yes           -y    Automatically accept prompts
    --no            -n    Automatically reject prompts
"
    }

    pub fn build(flags: &Vec<String>, config: &mut Config) -> Result<Self, String> {
        let mut options = Self::default();

        for flag in flags {
            let opt_val = &mut flag.split('=');
            let (opt, val) = (opt_val.next().unwrap(), opt_val.next());
            match opt {
                // Arguments requiring a value (--argument=value)
                "--install-dir" => config.install_dir = val.unwrap().to_string(),
                "--cache-timeout" => {
                    config.cache_timeout = val.unwrap().parse::<u64>().map_err(|e| e.to_string())?
                }

                opt if val.is_some() => return Err(format!("Unknown argument: {opt}=…")),

                // Arguments not requiring a value (--argument)
                "--reinstall" => options.reinstall = true,
                "--refresh" => options.refresh = true,
                "--cache-only" | "--no-refresh" => {
                    config.cache_timeout = u64::MAX;
                    options.refresh = false;
                }
                "--verbose" => options.verbose = true,
                "--force" => options.force = true,
                "--yes" => options.answer = Some(true),
                "--no" => options.answer = Some(false),

                // Short arguments and unknown argument errors
                opt => {
                    let mut opts = opt.chars();
                    if opts.next().unwrap() == '-' {
                        for o in opts {
                            match o {
                                'i' => options.reinstall = true,
                                'r' => options.refresh = true,
                                'c' => {
                                    config.cache_timeout = u64::MAX;
                                    options.refresh = false;
                                }
                                'v' => options.verbose = true,
                                'f' => options.force = true,
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

        Ok(options)
    }
}
