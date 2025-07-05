use fin::args::{show_help, Action, Args};
use fin::installed::InstalledFonts;
use fin::run;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut installed_fonts = InstalledFonts::read()?;
    let args = Args::build(&mut installed_fonts)?;

    match args.action {
        Action::Help => show_help(),
        _ => run(&args, &mut installed_fonts).map_err(|e| {
            let _ = installed_fonts.write();
            e
        })?,
    };

    Ok(())
}
