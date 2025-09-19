const BAR_SIZE: usize = 20;
const BAR_SIZE_F64: f64 = BAR_SIZE as f64;

pub fn show_progress(prefix: &str, progress: f64, suffix: &str) {
    use std::io::{stdout, Write};

    let cur_pos = (progress * BAR_SIZE_F64).round() as usize;
    let remainder = BAR_SIZE.saturating_sub(cur_pos);

    let bar = match progress < 0.999 && cur_pos > 0 {
        true => "—".repeat(cur_pos - 1) + "→" + &" ".repeat(remainder),
        false => "—".repeat(cur_pos) + &" ".repeat(remainder),
    };

    print!("\r{prefix} [{bar}] {suffix}");
    let _ = stdout().flush();
}
