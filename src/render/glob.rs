//! IR → glob (§6.5). Globs are coarse: varying class columns collapse to `*`.

use crate::ir::{Class, Run};

/// Render the run list as a glob pattern.
pub fn render(runs: &[Run]) -> String {
    let mut out = String::new();
    for run in runs {
        match run {
            Run::Literal { text } => {
                for c in text.chars() {
                    out.push_str(&escape(c));
                }
            }
            Run::Enum { alts, .. } => {
                if alts.iter().all(|a| a.chars().count() == 1) {
                    out.push('[');
                    for a in alts {
                        out.push_str(&escape(a.chars().next().unwrap()));
                    }
                    out.push(']');
                } else {
                    // Multi-char enum has no faithful glob form; widen to `*`.
                    out.push('*');
                }
            }
            Run::Class {
                class, min, max, ..
            } => {
                if matches!(class, Class::Any) && *min == 1 && *max == Some(1) {
                    out.push('?');
                } else {
                    out.push('*');
                }
            }
        }
    }
    out
}

fn escape(c: char) -> String {
    if matches!(c, '*' | '?' | '[' | ']') {
        format!("[{c}]")
    } else {
        c.to_string()
    }
}
