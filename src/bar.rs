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
    #[must_use]
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
    #[must_use]
    pub fn new(prefix: &str) -> Self {
        Self {
            state: State::InProgress,
            prefix: prefix.to_owned() + &" ".repeat(PREFIX_LENGTH.saturating_sub(prefix.len())),
            last_len: 0,
        }
    }

    /// Redraws the progress bar with the specified progress and suffix
    ///
    /// Note: Overwrites the last line of the output.
    /// If the output shifts, the output will not be as expected.
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
            // Overwrite previous output using spaces
            " ".repeat(self.last_len.saturating_sub(output.len()))
        );
        let _ = stdout().flush();
        self.last_len = output.len();
    }

    /// Updates the progress bar state and status icon
    pub fn update_state(&mut self, state: State) {
        print!("\r{}", state.status_symbol());
        let _ = stdout().flush();
        self.state = state;
    }

    /// Changes the status symbol to indicate successful completion
    /// Progress bar should not be updated after calling this function
    pub fn pass(&mut self) {
        self.state = State::Passed;
        println!("\r{}", self.state.status_symbol());
    }

    /// Changes the status symbol to indicate a failed operation
    /// Progress bar should not be updated after calling this function
    pub fn fail(&mut self) {
        self.state = State::Failed;
        println!("\r{}", self.state.status_symbol());
    }
}
