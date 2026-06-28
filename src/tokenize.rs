//! §6.1 — segment an input string into maximal runs by character category.
//!
//! Categories, in priority order: DIGIT, LOWER, UPPER, SPACE, LITERAL. Adjacent
//! chars of the same category coalesce into one run; a LITERAL is a single
//! specific char and never merges with an adjacent (different) literal.

/// The category of a character / run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cat {
    Digit,
    Lower,
    Upper,
    Space,
    Literal,
}

/// A maximal run of same-category characters from one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub cat: Cat,
    pub text: String,
}

fn category(c: char) -> Cat {
    if c.is_ascii_digit() {
        Cat::Digit
    } else if c.is_ascii_lowercase() {
        Cat::Lower
    } else if c.is_ascii_uppercase() {
        Cat::Upper
    } else if c.is_ascii_whitespace() {
        Cat::Space
    } else {
        Cat::Literal
    }
}

/// Split `s` into runs. Non-ASCII chars fall into LITERAL (one run each) and do
/// not crash — Unicode-correct classes are a documented v2.
pub fn tokenize(s: &str) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::new();
    for c in s.chars() {
        let cat = category(c);
        // LITERAL chars never coalesce; same-category non-literals do.
        let merge = cat != Cat::Literal && out.last().is_some_and(|t| t.cat == cat);
        if merge {
            out.last_mut().unwrap().text.push(c);
        } else {
            out.push(Token {
                cat,
                text: c.to_string(),
            });
        }
    }
    out
}

/// The skeleton of an input: its sequence of run categories, plus the literal
/// chars (so `ORD-` and `ORD:` are different skeletons but `ORD-1` / `ORD-99`
/// are the same). Literal identity is part of the skeleton; class lengths/text
/// are not.
pub fn skeleton(tokens: &[Token]) -> Vec<(Cat, Option<char>)> {
    tokens
        .iter()
        .map(|t| {
            let lit = if t.cat == Cat::Literal {
                t.text.chars().next()
            } else {
                None
            };
            (t.cat, lit)
        })
        .collect()
}
