//! The intermediate representation: a synthesized pattern is a list of runs.
//!
//! Synthesis (`synth`) produces this; rendering (`render`) turns it into a
//! dialect-specific string. JSON output (`json`) is a direct serialization.

use serde::Serialize;

/// One segment of the synthesized pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Run {
    /// A fixed string that appeared identically in every input.
    Literal { text: String },
    /// A small finite set of observed alternatives, e.g. `(log|txt)`.
    Enum {
        alts: Vec<String>,
        capture: Option<Cap>,
    },
    /// A character class repeated `[min, max]` times. `max == None` is unbounded.
    Class {
        class: Class,
        min: usize,
        max: Option<usize>,
        capture: Option<Cap>,
    },
}

/// The character class a [`Run::Class`] generalizes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class {
    Digit,
    Lower,
    Upper,
    Alpha,
    Alnum,
    Word,
    Space,
    /// An explicit set of characters, e.g. `[aou]`. Stored sorted + deduped.
    Set(Vec<char>),
    Any,
}

/// A capture group request attached to a varying run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cap {
    pub name: Option<String>,
}

/// Why a run stopped where it did — surfaced by `--explain`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Why {
    pub summary: String,
    pub reason: String,
}

impl Run {
    /// A literal run never varies, so it is never wrapped in a capture group.
    pub fn is_literal(&self) -> bool {
        matches!(self, Run::Literal { .. })
    }

    pub fn set_capture(&mut self, cap: Option<Cap>) {
        match self {
            Run::Enum { capture, .. } | Run::Class { capture, .. } => *capture = cap,
            Run::Literal { .. } => {}
        }
    }
}

impl Class {
    /// The base classes whose membership is a simple predicate, tightest first.
    /// `Set` and `Any` are handled separately.
    pub fn contains(&self, c: char) -> bool {
        match self {
            Class::Digit => c.is_ascii_digit(),
            Class::Lower => c.is_ascii_lowercase(),
            Class::Upper => c.is_ascii_uppercase(),
            Class::Alpha => c.is_ascii_alphabetic(),
            Class::Alnum => c.is_ascii_alphanumeric(),
            Class::Word => c.is_ascii_alphanumeric() || c == '_',
            Class::Space => c.is_ascii_whitespace(),
            Class::Set(chars) => chars.contains(&c),
            Class::Any => true,
        }
    }
}
