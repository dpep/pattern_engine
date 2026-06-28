//! §6.3–6.4 — climb the generalization lattice to the tightest safe pattern.
//!
//! For each aligned column we build a ladder of candidate renderings from
//! tightest to loosest, then greedily pick the loosest rung (up to a ceiling)
//! that still excludes every negative. With no negatives the ceiling is the
//! observed class; with negatives the ceiling rises to `\w` and the rejects
//! themselves are the stopping condition.

use crate::align::{Column, align};
use crate::ir::{Cap, Class, Run};
use crate::tokenize::Cat;
use crate::verify;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Tight,
    Default,
    Loose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Anon,
    Named,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    pub enum_max: usize,
    pub capture: Option<CaptureMode>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: Mode::Default,
            enum_max: 12,
            capture: None,
        }
    }
}

/// A synthesized pattern: a single sequence, or an alternation of sequences
/// when the inputs split across multiple skeletons (§6.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Seq(Vec<Run>),
    Alt(Vec<Vec<Run>>),
}

/// Which lattice rung a candidate sits on (orders the ladder, sets the ceiling).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rung {
    Literal,
    Enum,
    CharAlign,
    ObservedClass,
    WiderWord,
}

pub fn synthesize(positives: &[String], negatives: &[String], opts: &Options) -> Pattern {
    let groups = align(positives);
    let mut seqs: Vec<Vec<Run>> = groups
        .iter()
        .map(|g| synth_group(&g.columns, negatives, opts))
        .collect();

    if seqs.len() == 1 {
        Pattern::Seq(seqs.pop().unwrap())
    } else {
        Pattern::Alt(seqs)
    }
}

fn synth_group(columns: &[Column], negatives: &[String], opts: &Options) -> Vec<Run> {
    let has_neg = !negatives.is_empty();
    let ladders: Vec<Vec<(Rung, Vec<Run>)>> =
        columns.iter().map(|c| ladder(c, opts, has_neg)).collect();

    // Greedy left→right: loosen each column to the loosest safe rung.
    let mut chosen: Vec<usize> = vec![0; ladders.len()];
    for col in 0..ladders.len() {
        let ceiling = ceiling_idx(&ladders[col], has_neg);
        for idx in (0..=ceiling).rev() {
            chosen[col] = idx;
            let candidate = assemble(&ladders, &chosen);
            if !verify::admits_any(&candidate, negatives) {
                break;
            }
            chosen[col] = 0; // fall back to tightest if nothing safe
        }
    }

    let mut runs = assemble(&ladders, &chosen);
    coalesce_literals(&mut runs);
    apply_captures(&mut runs, opts.capture);
    runs
}

fn assemble(ladders: &[Vec<(Rung, Vec<Run>)>], chosen: &[usize]) -> Vec<Run> {
    let mut runs = Vec::new();
    for (ladder, &idx) in ladders.iter().zip(chosen) {
        runs.extend(ladder[idx].1.iter().cloned());
    }
    runs
}

/// Highest ladder index we may climb to: `\w` with negatives, the observed
/// class otherwise.
fn ceiling_idx(ladder: &[(Rung, Vec<Run>)], has_neg: bool) -> usize {
    if has_neg {
        return ladder.len() - 1;
    }
    ladder
        .iter()
        .rposition(|(r, _)| *r != Rung::WiderWord)
        .unwrap_or(0)
}

/// Build a column's candidate ladder, tightest → loosest.
fn ladder(col: &Column, opts: &Options, has_neg: bool) -> Vec<(Rung, Vec<Run>)> {
    let mut out: Vec<(Rung, Vec<Run>)> = Vec::new();
    let distinct = distinct_sorted(&col.values);

    // A constant column stays literal — generalizing it would admit strictly
    // more, never help exclude a negative. Digit runs are the exception: they
    // are the varying "values" the tool exists to erase, so `7` → `\d+` even
    // from a single example, while `ORD` / `id` stay literal.
    if distinct.len() == 1 && col.cat != Cat::Digit {
        out.push((
            Rung::Literal,
            vec![Run::Literal {
                text: distinct[0].clone(),
            }],
        ));
        return out;
    }

    if distinct.len() >= 2 && distinct.len() <= opts.enum_max {
        out.push((
            Rung::Enum,
            vec![Run::Enum {
                alts: distinct.clone(),
                capture: None,
            }],
        ));
    }

    if let Some(runs) = char_align(&col.values) {
        out.push((Rung::CharAlign, runs));
    }

    let (min, max) = bounds(&col.values, opts.mode, has_neg);
    let base = base_class(&col.values);
    out.push((
        Rung::ObservedClass,
        vec![Run::Class {
            class: base.clone(),
            min,
            max,
            capture: None,
        }],
    ));

    if word_widenable(&base) {
        out.push((
            Rung::WiderWord,
            vec![Run::Class {
                class: Class::Word,
                min,
                max,
                capture: None,
            }],
        ));
    }

    out
}

/// §6.4 char-alignment: only when every value is the same length and at least
/// one position is constant (so the literal frame is meaningful).
fn char_align(values: &[String]) -> Option<Vec<Run>> {
    let cols: Vec<Vec<char>> = values.iter().map(|v| v.chars().collect()).collect();
    let len = cols.first()?.len();
    if cols.iter().any(|c| c.len() != len) || len == 0 {
        return None;
    }

    let mut positional: Vec<Run> = Vec::new();
    let mut any_constant = false;
    for pos in 0..len {
        let mut chars: Vec<char> = cols.iter().map(|c| c[pos]).collect();
        chars.sort_unstable();
        chars.dedup();
        if chars.len() == 1 {
            any_constant = true;
            positional.push(Run::Literal {
                text: chars[0].to_string(),
            });
        } else {
            positional.push(Run::Class {
                class: Class::Set(chars),
                min: 1,
                max: Some(1),
                capture: None,
            });
        }
    }
    if any_constant {
        coalesce_literals(&mut positional);
        Some(positional)
    } else {
        None
    }
}

/// Length bounds for a class rung. Negatives (or `--loose`) loosen to `+`/`*`;
/// `--tight` keeps the exact observed range; the default uses a span rule.
fn bounds(values: &[String], mode: Mode, has_neg: bool) -> (usize, Option<usize>) {
    let lens: Vec<usize> = values.iter().map(|v| v.chars().count()).collect();
    let lo = *lens.iter().min().unwrap_or(&0);
    let hi = *lens.iter().max().unwrap_or(&0);
    let unbounded = (if lo == 0 { 0 } else { 1 }, None);

    match mode {
        Mode::Loose => unbounded,
        Mode::Tight => (lo, Some(hi)),
        Mode::Default => {
            if has_neg || lens.len() == 1 {
                // NOTE: a single sample can't justify a fixed length; with
                // negatives we generalize bounds and let the rejects bound us.
                unbounded
            } else {
                match hi - lo {
                    // NOTE: span ≤ 1 keeps an exact range (\d{4}, \d{1,2});
                    // a wider span collapses to + (ORD-\d+). Heuristic, tunable.
                    0 => (lo, Some(lo)),
                    1 => (lo, Some(hi)),
                    _ => unbounded,
                }
            }
        }
    }
}

/// The tightest base class covering every char of every value.
fn base_class(values: &[String]) -> Class {
    let chars: Vec<char> = values.iter().flat_map(|v| v.chars()).collect();
    let all = |f: fn(char) -> bool| chars.iter().all(|&c| f(c));
    if all(|c| c.is_ascii_digit()) {
        Class::Digit
    } else if all(|c| c.is_ascii_lowercase()) {
        Class::Lower
    } else if all(|c| c.is_ascii_uppercase()) {
        Class::Upper
    } else if all(|c| c.is_ascii_whitespace()) {
        Class::Space
    } else if all(|c| c.is_ascii_alphabetic()) {
        Class::Alpha
    } else if all(|c| c.is_ascii_alphanumeric()) {
        Class::Alnum
    } else if all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Class::Word
    } else {
        let mut set: Vec<char> = chars.clone();
        set.sort_unstable();
        set.dedup();
        Class::Set(set)
    }
}

/// Can this class widen to `\w`? (Word itself doesn't widen further.)
fn word_widenable(base: &Class) -> bool {
    matches!(
        base,
        Class::Digit | Class::Lower | Class::Upper | Class::Alpha | Class::Alnum
    )
}

fn distinct_sorted(values: &[String]) -> Vec<String> {
    let mut v: Vec<String> = values.to_vec();
    v.sort();
    v.dedup();
    v
}

/// Merge adjacent literal runs into one.
fn coalesce_literals(runs: &mut Vec<Run>) {
    let mut out: Vec<Run> = Vec::with_capacity(runs.len());
    for run in runs.drain(..) {
        if let (Some(Run::Literal { text: prev }), Run::Literal { text }) = (out.last_mut(), &run) {
            prev.push_str(text);
        } else {
            out.push(run);
        }
    }
    *runs = out;
}

/// Wrap each varying run in a capture group (§-c). Literals never capture.
fn apply_captures(runs: &mut [Run], mode: Option<CaptureMode>) {
    let Some(mode) = mode else { return };
    let mut n = 0;
    for run in runs.iter_mut() {
        if run.is_literal() {
            continue;
        }
        n += 1;
        let name = match mode {
            CaptureMode::Anon => None,
            CaptureMode::Named => Some(format!("n{n}")),
        };
        run.set_capture(Some(Cap { name }));
    }
}
