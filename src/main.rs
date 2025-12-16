use std::fs;

use fin::paths::lock_file_path;

// TODO: Shell completions

fn main() -> Result<(), String> {
    let lock_state = fs::read_to_string(lock_file_path()).map_or_else(
        |_| None,
        |lock_state| match lock_state.is_empty() {
            true => None,
            false => Some(lock_state.trim().to_string()),
        },
    );

    fin::run(lock_state)
}
