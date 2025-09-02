macro_rules! readme_path {
    () => {
        env!("CARGO_MANIFEST_DIR").to_owned() + "/README.md"
    };
}

mod test_readme {
    #![cfg(test)]

    use std::fs;

    #[test]
    fn consistent_help_message() {
        let readme = fs::read_to_string(readme_path!())
            .expect(&format!("`{}` is not a valid path", readme_path!()));

        if !readme.contains(&fin::args::show_help()) {
            panic!("Help message in the README needs to be updated");
        }
    }

    #[test]
    fn consistent_config_file() {
        let readme = fs::read_to_string(readme_path!())
            .expect(&format!("`{}` is not a valid path", readme_path!()));

        if !readme.contains(&fin::default_config!()) {
            panic!("Example configuration in the README needs to be updated");
        }
    }
}
