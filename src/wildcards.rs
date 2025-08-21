use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

pub struct WildcardPattern {
    index: usize,
    jumpback_index: Option<usize>,
    bytes: Box<[u8]>,
    valid: bool,
}

impl WildcardPattern {
    #[must_use]
    const fn new(bytes: Box<[u8]>) -> Self {
        Self {
            index: 0,
            jumpback_index: None,
            bytes,
            valid: true,
        }
    }

    /// Checks the current character, modifies the index,
    /// and validates or invalidates self.
    ///
    /// Returns `true` if the pattern has been matched in full,
    /// or `false` if partial or invalid.
    fn check_next(&mut self, input_byte: u8) -> bool {
        'iter: loop {
            if self.bytes.is_empty() {
                self.valid = false;
            }
            if self.index + 1 > self.bytes.len() {
                if let Some(index) = self.jumpback_index {
                    self.index = index;
                } else {
                    self.valid = false;
                }
            }
            if !self.valid {
                break 'iter;
            }

            match self.bytes[self.index] {
                b'*' => {
                    let p_len = self.bytes.len();
                    if p_len == 1 {
                        return true;
                    }
                    if self.index < p_len - 1 {
                        self.jumpback_index = Some(self.index);
                        self.index += 1;
                    } else {
                        return true;
                    }
                }
                c if c == input_byte => {
                    self.index += 1;
                    break 'iter;
                }
                _ => {
                    if let Some(last_star) = self.jumpback_index {
                        self.index = last_star + 1;
                    } else {
                        self.valid = false;
                    }
                    break 'iter;
                }
            }
        }
        false
    }
}

/// Returns `true` if the pattern matches the input
///
/// Supported special characters:
///   '*' matches any number of any character(s)
#[must_use]
pub fn match_wildcard(input: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }

    let mut wc_pattern = WildcardPattern::new(pattern.bytes().collect());
    for input_byte in input.bytes() {
        if wc_pattern.check_next(input_byte) {
            return true;
        }
    }

    for i in wc_pattern.index..wc_pattern.bytes.len() {
        match wc_pattern.bytes[i] {
            b'*' => wc_pattern.index += 1,
            c => {
                if c == b'$' && wc_pattern.index == wc_pattern.bytes.len() - 1 {
                    break;
                }
                wc_pattern.valid = false;
            }
        }
    }

    wc_pattern.valid
}

/// Returns `true` if any of the patterns match the input
///
/// Supported special characters:
///   '*' matches any number of any character(s)
#[must_use]
pub fn match_any_wildcard(input: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }

    let mut wc_patterns: Vec<WildcardPattern> = patterns
        .iter()
        .map(|p| WildcardPattern::new(p.bytes().collect()))
        .collect();

    for input_byte in input.bytes() {
        for wc_pattern in &mut wc_patterns {
            if wc_pattern.check_next(input_byte) {
                return true;
            }
        }
    }

    for mut wc_pattern in wc_patterns {
        for i in wc_pattern.index..wc_pattern.bytes.len() {
            match wc_pattern.bytes[i] {
                b'*' => wc_pattern.index += 1,
                c => {
                    if c == b'$' && wc_pattern.index == wc_pattern.bytes.len() - 1 {
                        break;
                    }
                    wc_pattern.valid = false;
                }
            }
        }
        if wc_pattern.valid {
            return true;
        }
    }

    false
}

/// Returns `true` if any of the patterns match the input
/// This is a multi-threaded version of `match_any_wildcard()`
///
/// Supported special characters:
///   '*' matches any number of any character(s)
#[must_use]
pub fn match_any_wildcard_mt(input: Arc<str>, patterns: Arc<[String]>) -> bool {
    let mut threads = Vec::new();

    for i in 0..patterns.len() {
        let input = Arc::clone(&input);
        let patterns = Arc::clone(&patterns);

        threads.push(thread::spawn(move || match_wildcard(&input, &patterns[i])));
    }

    let mut i = 0;
    loop {
        if threads.is_empty() {
            break;
        }

        if threads[i].is_finished() {
            if threads.remove(i).join().unwrap() {
                return true;
            }
            continue;
        }
        i += 1;

        if i == threads.len() {
            i = 0;
        }
    }
    false
}

/// Returns a `HashMap` of all matches.
/// Key: original `String` pattern
/// Value: `Vec<String>` of matches
///
/// Supported special characters:
///   '*' matches any number of any character(s)
#[must_use]
pub fn match_wildcards_multi(
    inputs: &[String],
    patterns: &[String],
) -> HashMap<String, Vec<String>> {
    let mut matches = HashMap::<String, Vec<String>>::new();
    for pattern in patterns {
        for input in inputs {
            // TODO: Multi-threading?
            if !match_wildcard(input, pattern) {
                continue;
            }
            matches
                .entry(pattern.to_string())
                .and_modify(|h| h.push(input.to_string()))
                .or_insert_with(|| vec![input.to_string()]);
        }
    }
    matches
}

/// Returns a portion of the input text matched by the wildcard pattern
///
/// Supported special characters:
///   '*' matches any number of any characters
///   '^' matches the start of the input
///   '$' matches the end of the input
#[must_use]
pub fn wildcard_substring<'a>(input: &'a str, pattern: &str, exclude: &[u8]) -> Option<&'a str> {
    if pattern.is_empty() {
        return None;
    }
    if pattern == "*" {
        return Some(input);
    }
    let mut start: Option<usize> = None;
    let mut wc_pattern = WildcardPattern::new(pattern.bytes().collect());
    let input_bytes = input.bytes().collect::<Box<[u8]>>();
    for input_index in 0..input.len() {
        'iter: loop {
            match wc_pattern.bytes[wc_pattern.index] {
                b'*' => {
                    if exclude.contains(&input_bytes[input_index]) {
                        wc_pattern.jumpback_index = None;
                        wc_pattern.index = 0;
                        start = None;
                        break 'iter;
                    }
                    if wc_pattern.index < wc_pattern.bytes.len() - 1 {
                        wc_pattern.jumpback_index = Some(wc_pattern.index);
                        wc_pattern.index += 1;
                    } else {
                        return Some(&input[start.unwrap_or(input_index)..input.len()]);
                    }
                    if start.is_none() {
                        start = Some(input_index);
                    }
                }
                b'^' if input_index + wc_pattern.index == 0 => {
                    start = Some(0);
                    wc_pattern.index += 1;
                }
                c if c == input_bytes[input_index] => {
                    if wc_pattern.index == wc_pattern.bytes.len() - 1 {
                        return Some(&input[start.unwrap_or(input_index)..=input_index]);
                    }
                    if start.is_none() {
                        start = Some(input_index);
                    }
                    wc_pattern.index += 1;
                    break 'iter;
                }
                _ => {
                    if start.is_none() {
                        break 'iter;
                    }
                    if let Some(last_star) = wc_pattern.jumpback_index {
                        wc_pattern.index = last_star + 1;
                        break 'iter;
                    }
                    wc_pattern.jumpback_index = None;
                    wc_pattern.index = 0;
                    start = None;
                    break 'iter;
                }
            }
        }
    }
    for i in wc_pattern.index..wc_pattern.bytes.len() {
        match wc_pattern.bytes[i] {
            b'*' => wc_pattern.index += 1,
            c => {
                if c == b'$' && wc_pattern.index == wc_pattern.bytes.len() - 1 {
                    break;
                }
                return None;
            }
        }
    }
    start.map(|start| &input[start..input.len()])
}
