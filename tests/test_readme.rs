mod test_readme {
    #![cfg(test)]

    use std::fs;

    #[test]
    fn consistent_help_message() {
        let readme_path = env!("CARGO_MANIFEST_DIR").to_owned() + "/README.md";
        let readme = fs::read_to_string(&readme_path)
            .expect(&format!("`{readme_path}` is not a valid path"));

        if !readme.contains(&fin::args::show_help()) {
            panic!("Help message in the README needs to be updated");
        }
    }
}
