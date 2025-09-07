const BAR_SIZE: u8 = 20;

pub fn show_progress(prefix: &str, progress: f64, suffix: &str) {
    use std::io::{stdout, Write};

    let mut bar = String::new();
    for i in 0..BAR_SIZE {
        bar += match (i as f64 - (progress * BAR_SIZE as f64)).round() {
            x if x < 0.0 => "—",
            0.0 => "→",
            _ => " ",
        }
    }

    print!("\r{prefix} [{bar}] {suffix}");
    let _ = stdout().flush();
}
