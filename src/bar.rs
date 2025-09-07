const BAR_SIZE: usize = 20;

pub fn show_progress(prefix: &str, progress: f64, suffix: &str) {
    use std::io::{stdout, Write};

    let cur_pos = (progress * BAR_SIZE as f64).round() as usize;
    let remainder = BAR_SIZE - cur_pos;
    let offset = 1.min(remainder);

    let bar = "—".repeat(cur_pos - offset) + &"→".repeat(offset) + &" ".repeat(remainder);

    print!("\r{prefix} [{bar}] {suffix}");
    let _ = stdout().flush();
}
