use fin::action::Action;
use fin::args::{show_help, Args};
use fin::config;
use fin::installed::InstalledFonts;
use fin::run;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut installed_fonts = InstalledFonts::read()?;
    let args = Args::build(&mut installed_fonts)?;

    match args.action {
        Action::Help => {
            show_help();
        }
        Action::Init => {
            config::Config::write_default_config()?;
        }
        _ => run(&args, &mut installed_fonts).inspect_err(|_| {
            let _ = installed_fonts.write();
        })?,
    }

    // TODO: Call `installed_fonts.write()` when cancelling with ^C
    //       in case any changes have already been performed

    Ok(())
}
