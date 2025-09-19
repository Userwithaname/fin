use std::io::{stdout, Write};

const PREFIX_LENGTH: usize = 13;
const BAR_SIZE: usize = 20;
const BAR_SIZE_F64: f64 = BAR_SIZE as f64;

pub enum State {
    InProgress,
    Passed,
    Failed,
}

impl State {
    pub fn status_symbol(&self) -> String {
        match self {
            State::InProgress => "…".to_owned(),
            State::Passed => green!("✓"),
            State::Failed => red!("×"),
        }
    }
}

pub struct ProgressBar {
    state: State,
    prefix: String,
    last_len: usize,
}

impl ProgressBar {
    pub fn new(prefix: &str) -> Self {
        Self {
            state: State::InProgress,
            prefix: prefix.to_owned() + &" ".repeat(PREFIX_LENGTH - prefix.len()),
            last_len: 0,
        }
    }

    pub fn update_progress(&mut self, progress: f64, suffix: &str) {
        let cur_pos = (progress * BAR_SIZE_F64).round() as usize;
        let remainder = BAR_SIZE.saturating_sub(cur_pos);

        let bar = match progress < 0.999 && cur_pos > 0 {
            true => "—".repeat(cur_pos - 1) + "→" + &" ".repeat(remainder),
            false => "—".repeat(cur_pos) + &" ".repeat(remainder),
        };

        let output = format!(
            "\r{} {} [{bar}] {suffix}",
            self.state.status_symbol(),
            self.prefix
        );
        print!(
            "{output}{}",
            " ".repeat(self.last_len.saturating_sub(output.len()))
        );
        let _ = stdout().flush();
        self.last_len = output.len();
    }

    pub fn update_state(&mut self, state: State) {
        print!("\r{}", state.status_symbol());
        let _ = stdout().flush();
        self.state = state;
    }

    pub fn pass(&mut self) {
        self.state = State::Passed;
        println!("\r{}", self.state.status_symbol());
    }

    pub fn fail(&mut self) {
        self.state = State::Failed;
        println!("\r{}", self.state.status_symbol());
    }
}
