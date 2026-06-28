//! §11 — gold acceptance cases. Each asserts the exact rendered output for a
//! `(positives, negatives, flags)` triple. Determinism is part of the contract.

use pattern_engine::render::wrap::ForTarget;
use pattern_engine::render::{self, Dialect};
use pattern_engine::synth::{CaptureMode, Mode, Options, synthesize};
use pattern_engine::verify;

fn vec(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// Synthesize and render as a bare regex in `dialect`, asserting verify passes.
fn regex_out(pos: &[&str], neg: &[&str], opts: &Options, dialect: Dialect) -> String {
    let (positives, negatives) = (vec(pos), vec(neg));
    let pattern = synthesize(&positives, &negatives, opts);
    let report = verify::verify_pattern(&pattern, &positives, &negatives);
    assert!(report.ok, "verify failed for {pos:?} / {neg:?}: {report:?}");
    render::render_pattern(&pattern, dialect)
}

fn default_opts() -> Options {
    Options::default()
}

#[test]
fn order_id_climbs_to_unbounded_digits() {
    let out = regex_out(
        &["ORD-1001", "ORD-99", "ORD-7"],
        &[],
        &default_opts(),
        Dialect::Pcre,
    );
    assert_eq!(out, r"ORD-\d+");
}

#[test]
fn fixed_length_dates_keep_exact_bounds() {
    let out = regex_out(
        &["2024-01-15", "2025-12-03"],
        &[],
        &default_opts(),
        Dialect::Pcre,
    );
    assert_eq!(out, r"\d{4}-\d{2}-\d{2}");
}

#[test]
fn reject_tightens_extension_to_enum() {
    let out = regex_out(
        &["a.log", "b.txt"],
        &["c.bak"],
        &default_opts(),
        Dialect::Pcre,
    );
    assert_eq!(out, r"\w+\.(log|txt)");
}

#[test]
fn reject_forces_char_aligned_set() {
    let out = regex_out(
        &["cat", "cot", "cut"],
        &["cit"],
        &default_opts(),
        Dialect::Pcre,
    );
    assert_eq!(out, "c[aou]t");
}

#[test]
fn glob_collapses_varying_segment() {
    let positives = vec(&["src/a.rs", "src/b.rs"]);
    let pattern = synthesize(&positives, &[], &default_opts());
    let report = verify::verify_glob(
        match &pattern {
            pattern_engine::synth::Pattern::Seq(r) => r,
            _ => unreachable!(),
        },
        &positives,
        &[],
    );
    assert!(report.ok, "glob verify failed: {report:?}");
    assert_eq!(render::render_glob_pattern(&pattern), "src/*.rs");
}

#[test]
fn bre_expands_shorthand_and_escapes_braces() {
    let out = regex_out(&["v1", "v2", "v10"], &[], &default_opts(), Dialect::Bre);
    assert_eq!(out, r"v[0-9]\{1,2\}");
}

#[test]
fn named_capture_is_deterministic() {
    let opts = Options {
        capture: Some(CaptureMode::Named),
        ..Options::default()
    };
    let out = regex_out(&["id=7"], &[], &opts, Dialect::Pcre);
    assert_eq!(out, r"id=(?<n1>\d+)");
}

#[test]
fn for_awk_wraps_in_ere() {
    let positives = vec(&["error: x", "error: y"]);
    let pattern = synthesize(&positives, &[], &default_opts());
    let dialect = ForTarget::Awk.dialect().unwrap();
    let pat = render::render_pattern(&pattern, dialect);
    let line = render::wrap::wrap(ForTarget::Awk, &pat);
    assert_eq!(line, "awk '/error: [a-z]/'");
}

#[test]
fn tight_mode_prefers_exact_range() {
    let opts = Options {
        mode: Mode::Tight,
        ..Options::default()
    };
    let out = regex_out(&["ORD-1001", "ORD-99", "ORD-7"], &[], &opts, Dialect::Pcre);
    assert_eq!(out, r"ORD-\d{1,4}");
}
