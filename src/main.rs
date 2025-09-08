use core::error::Error;
use std::fs;

#[macro_use]
mod paths;

fn main() -> Result<(), Box<dyn Error>> {
    let lock_state = fs::read_to_string(lock_file_path!()).map_or_else(
        |_| None,
        |lock_state| match lock_state.is_empty() {
            true => None,
            false => Some(lock_state.trim().to_string()),
        },
    );

    fin::run(lock_state)
}
