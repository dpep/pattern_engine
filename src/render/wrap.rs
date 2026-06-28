//! §5 — `--for` consumer wrappers and shell escaping.
//!
//! A target selects a dialect (see [`ForTarget::dialect`]) and wraps the
//! rendered pattern in a paste-ready command line.

use super::Dialect;

/// A `--for` consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForTarget {
    Grep,
    Egrep,
    Sed,
    Awk,
    Perl,
    Rg,
    Glob,
}

impl ForTarget {
    pub fn parse(s: &str) -> Option<ForTarget> {
        Some(match s {
            "grep" => ForTarget::Grep,
            "egrep" => ForTarget::Egrep,
            "sed" => ForTarget::Sed,
            "awk" => ForTarget::Awk,
            "perl" => ForTarget::Perl,
            "rg" => ForTarget::Rg,
            "glob" => ForTarget::Glob,
            _ => return None,
        })
    }

    /// The dialect this consumer expects. `glob` has no regex dialect.
    pub fn dialect(self) -> Option<Dialect> {
        Some(match self {
            ForTarget::Grep | ForTarget::Sed => Dialect::Bre,
            ForTarget::Egrep | ForTarget::Awk => Dialect::Ere,
            ForTarget::Perl => Dialect::Pcre,
            ForTarget::Rg => Dialect::Re2,
            ForTarget::Glob => return None,
        })
    }
}

/// Wrap an already-rendered `pattern` into a paste-ready line for `target`.
pub fn wrap(target: ForTarget, pattern: &str) -> String {
    match target {
        ForTarget::Grep => format!("grep {}", quote(pattern)),
        ForTarget::Egrep => format!("grep -E {}", quote(pattern)),
        ForTarget::Sed => format!("sed -n {}", quote(&format!("/{pattern}/p"))),
        ForTarget::Awk => format!("awk {}", quote(&format!("/{pattern}/"))),
        ForTarget::Perl => format!("perl -ne {}", quote(&format!("print if /{pattern}/"))),
        ForTarget::Rg => format!("rg {}", quote(pattern)),
        ForTarget::Glob => pattern.to_string(),
    }
}

/// Single-quote for the shell, escaping embedded single quotes the shell way.
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
