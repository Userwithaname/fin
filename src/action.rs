use crate::show_help;

pub enum Action {
    Install,
    Update,
    Remove,
    List,
    Help,
}

impl Action {
    pub fn help_actions() -> &'static str {
        // Remember to update README.md
        "\
Actions:
    install               Install new fonts
    update                Update installed fonts
    remove                Remove installed fonts
    list                  List installed or available fonts
    help                  Show this help message
"
    }

    pub fn parse(action: Option<String>) -> Result<Self, String> {
        Ok(match action {
            Some(a) => match a.as_str() {
                "install" | "get" => Action::Install,
                "update" | "upgrade" | "up" => Action::Update,
                "remove" | "uninstall" | "rm" => Action::Remove,
                "list" | "ls" => Action::List,
                "help" | "h" => Action::Help,
                _ => {
                    show_help();
                    println!();
                    return Err(format!("Unrecognized action: {a}"));
                }
            },
            None => Action::Help,
        })
    }
}
