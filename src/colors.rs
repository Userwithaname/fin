#[macro_export]
macro_rules! green {
    ($contents:expr) => {
        ["\x1b[92m", $contents, "\x1b[0m"].concat()
    };
}
#[macro_export]
macro_rules! format_green {
    ($contents:literal) => {
        format!("\x1b[92m{}\x1b[0m", format!($contents))
    };
}
#[macro_export]
macro_rules! println_green {
    ($contents:literal) => {
        println!("\x1b[92m{}\x1b[0m", format!($contents))
    };
}

#[macro_export]
macro_rules! orange {
    ($contents:expr) => {
        ["\x1b[93m", $contents, "\x1b[0m"].concat()
    };
}
#[macro_export]
macro_rules! format_orange {
    ($contents:literal) => {
        format!("\x1b[93m{}\x1b[0m", format!($contents))
    };
}
#[macro_export]
macro_rules! println_orange {
    ($contents:literal) => {
        println!("\x1b[93m{}\x1b[0m", format!($contents))
    };
}

#[macro_export]
macro_rules! red {
    ($contents:expr) => {
        ["\x1b[91m", $contents, "\x1b[0m"].concat()
    };
}
#[macro_export]
macro_rules! format_red {
    ($contents:literal) => {
        format!("\x1b[91m{}\x1b[0m", format!($contents))
    };
}
#[macro_export]
macro_rules! println_red {
    ($contents:literal) => {
        println!("\x1b[91m{}\x1b[0m", format!($contents))
    };
}
