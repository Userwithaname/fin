mod test_readme {
    #![cfg(test)]

    use std::fs;

    #[inline]
    fn readme() -> String {
        let readme_path = env!("CARGO_MANIFEST_DIR").to_owned() + "/README.md";
        fs::read_to_string(&readme_path).expect(&format!("`{readme_path}` is not a valid path"))
    }

    #[test]
    fn consistent_help_message() {
        if !readme().contains(&fin::args::show_help()) {
            panic!("Help message in the README needs to be updated");
        }
    }

    #[test]
    fn consistent_config_file() {
        let default_config = &fin::default_config!();
        println!("{default_config}");
        if !readme().contains(default_config) {
            panic!("Example configuration in the README needs to be updated");
        }
    }
}
