pub struct WildcardPattern {
    index: usize,
    jumpback_index: Option<usize>,
    bytes: Box<[u8]>,
    valid: bool,
}

/// Returns true if the input matches any of the patterns
///
/// Supported special characters:
///   '*' matches any number of any characters
///   '^' matches the start of the input
///   '$' matches the end of the input
pub fn match_any_wildcard(input: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let mut wc_patterns: Vec<WildcardPattern> = patterns
        .iter()
        .map(|p| WildcardPattern {
            index: 0,
            jumpback_index: None,
            bytes: p.bytes().collect(),
            valid: true,
        })
        .collect();

    for input_byte in input.bytes() {
        for wc_pattern in &mut wc_patterns {
            'iter: loop {
                if wc_pattern.bytes.is_empty() {
                    wc_pattern.valid = false;
                }
                if wc_pattern.index + 1 > wc_pattern.bytes.len() {
                    if let Some(index) = wc_pattern.jumpback_index {
                        wc_pattern.index = index;
                    } else {
                        wc_pattern.valid = false;
                    }
                }
                if !wc_pattern.valid {
                    break 'iter;
                }

                match wc_pattern.bytes[wc_pattern.index] {
                    b'*' => {
                        let p_len = wc_pattern.bytes.len();
                        if p_len == 1 {
                            return true;
                        }
                        if wc_pattern.index < p_len - 1 {
                            wc_pattern.jumpback_index = Some(wc_pattern.index);
                            wc_pattern.index += 1;
                        } else {
                            return true;
                        }
                    }
                    // b'^' if input_index + wc_pattern.index == 0 => {
                    //     wc_pattern.index += 1;
                    // }
                    c if c == input_byte => {
                        wc_pattern.index += 1;
                        break 'iter;
                    }
                    _ => {
                        if let Some(last_star) = wc_pattern.jumpback_index {
                            wc_pattern.index = last_star + 1;
                        } else {
                            wc_pattern.valid = false;
                        }
                        break 'iter;
                    }
                }
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

/// Returns the portion of the input text matching the wildcard pattern
///
/// Supported special characters:
///   '*' matches any number of any characters
///   '^' matches the start of the input
///   '$' matches the end of the input
pub fn wildcard_substring<'a>(input: &'a str, pattern: &str, exclude: &[u8]) -> Option<&'a str> {
    if pattern.is_empty() {
        return None;
    }
    if pattern == "*" {
        return Some(input);
    }
    let mut start: Option<usize> = None;
    let mut wc_pattern = WildcardPattern {
        index: 0,
        jumpback_index: None,
        bytes: pattern.bytes().collect(),
        valid: true,
    };
    let input_bytes = input.bytes().collect::<Box<[u8]>>();
    for input_index in 0..input.len() {
        'iter: loop {
            match wc_pattern.bytes[wc_pattern.index] {
                b'*' => {
                    if start.is_none() {
                        start = Some(input_index);
                    }
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
                        return Some(&input[start.unwrap()..input.len()]);
                    }
                }
                b'^' if input_index + wc_pattern.index == 0 => {
                    start = Some(0);
                    wc_pattern.index += 1;
                }
                c if c == input_bytes[input_index] => {
                    if start.is_none() {
                        start = Some(input_index);
                    }
                    if wc_pattern.index == wc_pattern.bytes.len() - 1 {
                        return Some(&input[start.unwrap()..=input_index]);
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
    match start {
        Some(start) => Some(&input[start..input.len()]),
        None => None,
    }
}
