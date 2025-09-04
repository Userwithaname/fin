use fin::action::Action;
use fin::args::{show_help, Args};
use fin::config;
use fin::font::Font;
use fin::installed::InstalledFonts;
use fin::run;
use std::error::Error;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn Error>> {
    let mut installed_fonts = InstalledFonts::read()?;
    let (args, items) = Args::build()?;

    match args.action {
        Action::Help => {
            show_help();
        }
        Action::Init => {
            config::Config::write_default_config()?;
        }
        _ => {
            let mut fonts: Box<[Font]> =
                Font::get_actionable_fonts(Arc::new(args.clone()), &items, &installed_fonts)
                    .map_err(|e| e.to_string())?
                    .into();
            run(&args, &mut fonts, &mut installed_fonts).inspect_err(|_| {
                let _ = installed_fonts.write();
            })?
        }
    }

    // TODO: Call `installed_fonts.write()` when cancelling with ^C
    //       in case any changes have already been performed

    Ok(())
}
