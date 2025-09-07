const BAR_SIZE: usize = 20;

pub fn show_progress(prefix: &str, progress: f64, suffix: &str) {
    use std::io::{stdout, Write};

    let cur_pos = (progress * BAR_SIZE as f64).round() as usize;
    let remainder = match BAR_SIZE > cur_pos {
        true => BAR_SIZE - cur_pos,
        false => 0,
    };

    let bar = match progress < 0.995 && cur_pos > 0 {
        true => "—".repeat(cur_pos - 1) + &"→" + &" ".repeat(remainder),
        false => "—".repeat(cur_pos) + &" ".repeat(remainder),
    };

    print!("\r{prefix} [{bar}] {suffix}");
    let _ = stdout().flush();
}
