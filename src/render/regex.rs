//! IR → regex string, per dialect (§6.5).

use super::Dialect;
use crate::ir::{Cap, Class, Run};

/// Render the run list as a regex in `dialect`.
pub fn render(runs: &[Run], dialect: Dialect) -> String {
    let mut out = String::new();
    for run in runs {
        out.push_str(&render_run(run, dialect));
    }
    out
}

fn render_run(run: &Run, dialect: Dialect) -> String {
    match run {
        Run::Literal { text } => text.chars().map(|c| escape_literal(c, dialect)).collect(),
        Run::Enum { alts, capture } => {
            let body = alts
                .iter()
                .map(|a| {
                    a.chars()
                        .map(|c| escape_literal(c, dialect))
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join(alt(dialect));
            group(&body, capture, dialect)
        }
        Run::Class {
            class,
            min,
            max,
            capture,
        } => {
            let atom = format!(
                "{}{}",
                class_atom(class, dialect),
                quantifier(*min, *max, dialect)
            );
            if capture.is_some() {
                group(&atom, capture, dialect)
            } else {
                atom
            }
        }
    }
}

/// Wrap `body` in a group: a bare alternation needs one to bind; a capture
/// request makes it `( )` or `(?<name> )`.
fn group(body: &str, capture: &Option<Cap>, dialect: Dialect) -> String {
    if dialect.is_bre() {
        // BRE has no named groups; `\(...\)` captures, `\|` alternates.
        return format!("\\({body}\\)");
    }
    match capture {
        Some(Cap { name: Some(n) }) => format!("(?<{n}>{body})"),
        _ => format!("({body})"),
    }
}

fn alt(dialect: Dialect) -> &'static str {
    if dialect.is_bre() { "\\|" } else { "|" }
}

fn class_atom(class: &Class, dialect: Dialect) -> String {
    let sh = dialect.shorthand();
    match class {
        Class::Digit if sh => "\\d".into(),
        Class::Digit => "[0-9]".into(),
        Class::Lower => "[a-z]".into(),
        Class::Upper => "[A-Z]".into(),
        Class::Alpha => "[A-Za-z]".into(),
        Class::Alnum => "[A-Za-z0-9]".into(),
        Class::Word if sh => "\\w".into(),
        Class::Word => "[A-Za-z0-9_]".into(),
        Class::Space if sh => "\\s".into(),
        Class::Space => "[[:space:]]".into(),
        Class::Set(chars) => {
            let inner: String = chars.iter().map(|c| escape_in_class(*c)).collect();
            format!("[{inner}]")
        }
        Class::Any => ".".into(),
    }
}

fn quantifier(min: usize, max: Option<usize>, dialect: Dialect) -> String {
    let bre = dialect.is_bre();
    let braces = |s: String| {
        if bre {
            format!("\\{{{s}\\}}")
        } else {
            format!("{{{s}}}")
        }
    };
    match (min, max) {
        (1, Some(1)) => String::new(),
        (n, Some(m)) if n == m => braces(format!("{n}")),
        (n, Some(m)) => braces(format!("{n},{m}")),
        (0, None) => "*".into(),
        // BRE has no `+`; spell unbounded as `\{n,\}`.
        (1, None) if bre => "\\{1,\\}".into(),
        (1, None) => "+".into(),
        (n, None) if bre => format!("\\{{{n},\\}}"),
        (n, None) => braces(format!("{n},")),
    }
}

/// Escape a literal char for use outside a character class.
fn escape_literal(c: char, dialect: Dialect) -> String {
    let special: &[char] = if dialect.is_bre() {
        &['.', '*', '[', ']', '\\', '^', '$']
    } else {
        &[
            '.', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '\\',
        ]
    };
    if special.contains(&c) {
        format!("\\{c}")
    } else {
        c.to_string()
    }
}

/// Escape a char for use inside `[...]`.
fn escape_in_class(c: char) -> String {
    if matches!(c, ']' | '\\' | '^' | '-') {
        format!("\\{c}")
    } else {
        c.to_string()
    }
}
