//! Read positive / negative example strings from files or stdin.
//!
//! One example per line; blank lines and a trailing newline are ignored. No
//! files means read positives from stdin (negatives only ever come from `-v`).

use std::fs;
use std::io::{self, Read};

use anyhow::{Context, Result};

/// Read examples from the given files, or from stdin if `files` is empty.
pub fn read(files: &[String], allow_stdin: bool) -> Result<Vec<String>> {
    let mut raw = String::new();
    if files.is_empty() {
        if allow_stdin {
            io::stdin()
                .read_to_string(&mut raw)
                .context("reading stdin")?;
        }
    } else {
        for f in files {
            let body = fs::read_to_string(f).with_context(|| format!("reading {f}"))?;
            raw.push_str(&body);
            if !body.ends_with('\n') {
                raw.push('\n');
            }
        }
    }
    Ok(raw
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}
