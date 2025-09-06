mod test_action {
    #![cfg(test)]

    use fin::action::Action;
    use serde::Deserialize;

    /// https://users.rust-lang.org/t/ensure-exhaustiveness-of-list-of-enum-variants/99891/3
    macro_rules! ensure_exhaustive {
        ($E:path, $($variant:ident),*) => {
            {
                use $E as E;
                let _ = |dummy: E| {
                    match dummy {
                        $(E::$variant => ()),*
                    }
                };
                [$(E::$variant),*]
            }
        }
    }

    #[test]
    fn help_includes_all_actions() {
        let all_actions = ensure_exhaustive!(
            Action, Install, Reinstall, Update, Remove, List, Clean, Config, Version, Help
        );
        let help_actions = Action::help_actions();
        print!("{help_actions}");

        all_actions.iter().for_each(|action| {
            let action = format!("  {action:?} ").to_lowercase();
            if !help_actions.contains(&action) {
                panic!(
                    "The '{}' action is missing from the help message",
                    action.trim()
                );
            }
        });
    }

    #[derive(Deserialize)]
    struct CargoInfo {
        package: CargoPackage,
    }

    #[derive(Deserialize)]
    struct CargoPackage {
        version: String,
    }

    #[test]
    fn consistent_version_numbers() {
        use std::fs;

        let cargo_toml_path = env!("CARGO_MANIFEST_DIR").to_owned() + "/Cargo.toml";
        let cargo_info: CargoInfo =
            toml::from_str(&fs::read_to_string(cargo_toml_path).unwrap()).unwrap();
        let cmp_version = cargo_info.package.version;

        println!(
            "\
Version:    {}
Cargo.toml: {}",
            fin::VERSION,
            cmp_version
        );

        assert!(
            fin::VERSION == cmp_version,
            "Version number reported by the program differs from Cargo.toml"
        );
    }
}
