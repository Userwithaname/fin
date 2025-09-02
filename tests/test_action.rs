mod test_actions {
    #![cfg(test)]

    use fin::action::Action;

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
        let all_actions =
            ensure_exhaustive!(Action, Install, Reinstall, Update, Remove, List, Clean, Init, Help);
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
}
