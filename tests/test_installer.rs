mod test_installer {
    use fin::config::Config;
    use fin::font_page::FontPage;
    use fin::installer::Installer;
    use fin::options::Options;
    use fin::{cache_dir, home_dir, page_cache_dir};
    use std::collections::HashMap;
    use std::fs;
    use std::sync::{Arc, Mutex};

    #[test]
    fn valid_default_installers() {
        let installer_dir = env!("CARGO_MANIFEST_DIR").to_owned() + "/installers/";

        let cached_pages = Arc::new(Mutex::new(HashMap::<String, FontPage>::new()));
        let _ = fs::create_dir_all(page_cache_dir!()).inspect_err(|_| {
            panic!("Failed to create directory: {}", page_cache_dir!());
        });

        let args = Arc::new(fin::args::Args {
            action: fin::action::Action::Install,
            config: Config {
                cache_timeout: u64::MAX,
                ..Config::default()
            },
            options: Options::default(),
        });

        for file in fs::read_dir(&installer_dir).unwrap() {
            if let Err(e) = Installer::parse(
                &args,
                &installer_dir,
                &file.unwrap().file_name().into_string().unwrap(),
                None,
                Arc::clone(&cached_pages),
            ) {
                panic!("{e}");
            }
        }
    }
}
