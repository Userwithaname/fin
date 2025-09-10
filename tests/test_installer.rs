mod test_installer {
    use fin::config::Config;
    use fin::font_page::FontPage;
    use fin::installer::Installer;
    use fin::options::Options;
    use std::collections::HashMap;
    use std::fs;
    use std::sync::{Arc, Mutex};

    #[test]
    fn valid_default_installers() {
        let installer_dir = env!("CARGO_MANIFEST_DIR").to_owned() + "/installers/";
        let cached_pages = Arc::new(Mutex::new(HashMap::<u64, FontPage>::new()));
        for file in fs::read_dir(&installer_dir).unwrap() {
            if let Err(e) = Installer::parse(
                Arc::new(fin::args::Args {
                    action: fin::action::Action::Install,
                    config: Config {
                        cache_timeout: u64::MAX,
                        ..Config::default()
                    },
                    options: Options {
                        reinstall: true,
                        refresh: false,
                        verbose: false,
                        answer: None,
                        force: false,
                    },
                }),
                installer_dir.clone(),
                &file.unwrap().file_name().into_string().unwrap(),
                None,
                Arc::clone(&cached_pages),
            ) {
                panic!("{e}");
            }
        }
    }
}
