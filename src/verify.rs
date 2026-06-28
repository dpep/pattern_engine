//! §8 — every emitted pattern is re-checked against all inputs.
//!
//! The climb (`synth`) also uses [`matches`] to decide whether a generalization
//! step would admit a negative. Verification is semantic: we compile the IR in
//! the canonical (regex-crate) flavor and full-match. BRE/ERE outputs are a
//! faithful transform of the same IR, so this is their ERE-equivalent check.

use ::regex::Regex;
use globset::{Glob, GlobMatcher};

use crate::ir::Run;
use crate::render::regex as regex_render;
use crate::render::{self, Dialect, glob};
use crate::synth::Pattern;

/// Compile the run list into an anchored, full-string regex matcher.
fn compile(runs: &[Run]) -> Option<Regex> {
    let body = regex_render::render(runs, Dialect::Re2);
    Regex::new(&format!("^(?:{body})$")).ok()
}

/// Does the synthesized pattern fully match `s`?
pub fn matches(runs: &[Run], s: &str) -> bool {
    compile(runs).is_some_and(|re| re.is_match(s))
}

/// Would any negative be admitted by this pattern?
pub fn admits_any(runs: &[Run], negatives: &[String]) -> bool {
    match compile(runs) {
        Some(re) => negatives.iter().any(|n| re.is_match(n)),
        None => true, // uncompilable → treat as unsafe
    }
}

/// Outcome of the final safety check.
#[derive(Debug)]
pub struct Report {
    pub ok: bool,
    pub unmatched_positives: Vec<String>,
    pub admitted_negatives: Vec<String>,
}

/// Verify the regex IR against all inputs.
pub fn verify(runs: &[Run], positives: &[String], negatives: &[String]) -> Report {
    let re = compile(runs);
    let (unmatched_positives, admitted_negatives) = match &re {
        Some(re) => (
            positives
                .iter()
                .filter(|p| !re.is_match(p))
                .cloned()
                .collect(),
            negatives
                .iter()
                .filter(|n| re.is_match(n))
                .cloned()
                .collect(),
        ),
        None => (positives.to_vec(), negatives.to_vec()),
    };
    Report {
        ok: re.is_some() && unmatched_positives.is_empty() && admitted_negatives.is_empty(),
        unmatched_positives,
        admitted_negatives,
    }
}

/// Verify a whole pattern (handles the multi-skeleton alternation) against all
/// inputs, using the canonical regex flavor.
pub fn verify_pattern(pattern: &Pattern, positives: &[String], negatives: &[String]) -> Report {
    let body = render::render_pattern(pattern, Dialect::Re2);
    let re = Regex::new(&format!("^(?:{body})$")).ok();
    let (unmatched_positives, admitted_negatives) = match &re {
        Some(re) => (
            positives
                .iter()
                .filter(|p| !re.is_match(p))
                .cloned()
                .collect(),
            negatives
                .iter()
                .filter(|n| re.is_match(n))
                .cloned()
                .collect(),
        ),
        None => (positives.to_vec(), negatives.to_vec()),
    };
    Report {
        ok: re.is_some() && unmatched_positives.is_empty() && admitted_negatives.is_empty(),
        unmatched_positives,
        admitted_negatives,
    }
}

fn glob_matcher(runs: &[Run]) -> Option<GlobMatcher> {
    Glob::new(&glob::render(runs))
        .ok()
        .map(|g| g.compile_matcher())
}

/// Verify a glob rendering against all inputs (uses `globset`, not regex).
pub fn verify_glob(runs: &[Run], positives: &[String], negatives: &[String]) -> Report {
    let m = glob_matcher(runs);
    let (unmatched_positives, admitted_negatives) = match &m {
        Some(m) => (
            positives
                .iter()
                .filter(|p| !m.is_match(p))
                .cloned()
                .collect(),
            negatives
                .iter()
                .filter(|n| m.is_match(n))
                .cloned()
                .collect(),
        ),
        None => (positives.to_vec(), negatives.to_vec()),
    };
    Report {
        ok: m.is_some() && unmatched_positives.is_empty() && admitted_negatives.is_empty(),
        unmatched_positives,
        admitted_negatives,
    }
}
