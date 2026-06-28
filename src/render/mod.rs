//! §6.5 — turn the IR into a concrete pattern string.
//!
//! `regex` renders for the regex dialects; `glob` renders globs; `wrap` adds
//! the paste-ready `--for` wrappers and shell escaping.

pub mod glob;
pub mod regex;
pub mod wrap;

use crate::synth::Pattern;

/// Render a synthesized pattern as a regex in `dialect`, handling the
/// multi-skeleton alternation case.
pub fn render_pattern(pattern: &Pattern, dialect: Dialect) -> String {
    match pattern {
        Pattern::Seq(runs) => regex::render(runs, dialect),
        Pattern::Alt(groups) => {
            let bodies: Vec<String> = groups.iter().map(|g| regex::render(g, dialect)).collect();
            if dialect.is_bre() {
                format!("\\({}\\)", bodies.join("\\|"))
            } else {
                format!("({})", bodies.join("|"))
            }
        }
    }
}

/// Render a synthesized pattern as a glob.
pub fn render_glob_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Seq(runs) => glob::render(runs),
        // globset spells alternation as `{a,b}`.
        Pattern::Alt(groups) => {
            let bodies: Vec<String> = groups.iter().map(|g| glob::render(g)).collect();
            format!("{{{}}}", bodies.join(","))
        }
    }
}

/// Regex syntax flavor. Selected by `--dialect` (or implied by `--for`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Pcre,
    Ere,
    Bre,
    Re2,
    Python,
    Js,
}

impl Dialect {
    pub fn parse(s: &str) -> Option<Dialect> {
        Some(match s {
            "pcre" => Dialect::Pcre,
            "ere" => Dialect::Ere,
            "bre" => Dialect::Bre,
            "re2" => Dialect::Re2,
            "python" => Dialect::Python,
            "js" => Dialect::Js,
            _ => return None,
        })
    }

    /// Does this dialect understand `\d` / `\w` / `\s` shorthand? POSIX
    /// BRE/ERE do not — they get bracket expansions instead.
    pub fn shorthand(self) -> bool {
        !matches!(self, Dialect::Ere | Dialect::Bre)
    }

    /// BRE makes `+ ? ( ) { } |` literal unless backslash-escaped.
    pub fn is_bre(self) -> bool {
        matches!(self, Dialect::Bre)
    }
}
