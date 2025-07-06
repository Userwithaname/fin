mod config_tests {
    #![cfg(test)]

    use fin::config::Config;
    use fin::default_config;

    #[test]
    fn valid_default_config() {
        let cmp_lines = toml::to_string(&Config::default()).unwrap();
        let cmp_lines = cmp_lines.lines();

        println!(
            "Expected:\n{}\nProvided:\n{}",
            cmp_lines
                .clone()
                .map(|line| " > ".to_owned() + line + "\n")
                .collect::<String>(),
            default_config!()
                .lines()
                .map(|line| " > ".to_owned() + line + "\n")
                .collect::<String>()
        );

        let mut count = 0;
        for line in default_config!().lines() {
            for cmp_line in cmp_lines.clone() {
                let mut components = cmp_line.split('=');
                if let (Some(key), Some(value)) = (components.next(), components.next()) {
                    let cmp = &format!("{key}={value}");
                    if line.replace("# ", "").starts_with(cmp) {
                        count += 1;
                    } else if line.starts_with(key) {
                        println!(
                            "Non-match:\n	{} <--- default_config!()\n	{} <--- Config::default()",
                            line, cmp
                        );
                    }
                }
            }
        }
        if count != cmp_lines.count() {
            panic!("The default config in `default_config!()` macro has missing fields.");
        }
    }
}
