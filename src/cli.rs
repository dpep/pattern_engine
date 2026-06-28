//! Command-line surface (§4): wire cli → input → synth → render → print.

use std::process::ExitCode;

use clap::Parser;

use crate::ir::{Class, Run};
use crate::render::wrap::ForTarget;
use crate::render::{self, Dialect};
use crate::synth::{CaptureMode, Mode, Options, Pattern, synthesize};
use crate::{input, json, verify};

/// Read example strings, emit the tightest pattern that matches them — and,
/// given counter-examples (`-v`), nothing else.
#[derive(Debug, Parser)]
#[command(name = "pe", version, about)]
struct Cli {
    /// Positive example files (stdin if none).
    files: Vec<String>,

    /// Negative example files — the pattern must NOT match these.
    #[arg(short = 'v', long = "reject", value_name = "FILE")]
    reject: Vec<String>,

    /// Emit a runnable line for a consumer: grep egrep sed awk perl rg glob.
    #[arg(long = "for", value_name = "TARGET")]
    for_target: Option<String>,

    /// Regex flavor: pcre ere bre re2 python js.
    #[arg(long = "dialect", default_value = "pcre", value_name = "FLAVOR")]
    dialect: String,

    /// Keep exact observed bounds; prefer literals and enums.
    #[arg(long = "tight", conflicts_with = "loose")]
    tight: bool,

    /// Collapse bounds to +/*, widen classes sooner.
    #[arg(long = "loose")]
    loose: bool,

    /// Wrap varying runs in capture groups: -c or -c=named.
    #[arg(
        short = 'c',
        long = "capture",
        num_args = 0..=1,
        default_missing_value = "anon",
        require_equals = true,
        value_name = "named",
    )]
    capture: Option<String>,

    /// Explain each run: the class chosen and why it stopped there.
    #[arg(short = 'e', long = "explain")]
    explain: bool,

    /// Emit N ranked candidates (tightest → loosest).
    #[arg(short = 'l', long = "limit", value_name = "N")]
    limit: Option<usize>,

    /// Structured IR as JSON.
    #[arg(short = 'j', long = "json", conflicts_with = "ndjson")]
    json: bool,

    /// Structured IR as NDJSON (one run per line).
    #[arg(short = 'J', long = "ndjson")]
    ndjson: bool,

    /// Print pass/fail of the verify step on stderr, then the pattern.
    #[arg(long = "test")]
    test: bool,
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    match execute(&cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("pe: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn execute(cli: &Cli) -> anyhow::Result<ExitCode> {
    let positives = input::read(&cli.files, true)?;
    let negatives = input::read(&cli.reject, false)?;
    if positives.is_empty() {
        anyhow::bail!("no positive examples (give files or pipe stdin)");
    }

    let for_target = cli
        .for_target
        .as_deref()
        .map(|s| ForTarget::parse(s).ok_or_else(|| anyhow::anyhow!("unknown --for target: {s}")))
        .transpose()?;
    let is_glob = for_target == Some(ForTarget::Glob);

    let mut opts = Options {
        mode: if cli.tight {
            Mode::Tight
        } else if cli.loose {
            Mode::Loose
        } else {
            Mode::Default
        },
        enum_max: 12,
        capture: parse_capture(cli.capture.as_deref())?,
    };
    if is_glob && opts.capture.is_some() {
        eprintln!("pe: -c is meaningless for --for glob; ignoring");
        opts.capture = None;
    }

    let pattern = synthesize(&positives, &negatives, &opts);

    // Structured output short-circuits rendering.
    if cli.json {
        println!("{}", json::to_json(&pattern));
        return Ok(ExitCode::SUCCESS);
    }
    if cli.ndjson {
        println!("{}", json::to_ndjson(&pattern));
        return Ok(ExitCode::SUCCESS);
    }

    if cli.limit.is_some_and(|n| n > 1) {
        // NOTE: ranked multi-candidate emission (§9) is not yet implemented;
        // the single tightest-safe pattern is printed.
        eprintln!("pe: --limit > 1 not yet implemented; emitting the single best pattern");
    }

    // Render + verify.
    let (output, report) = if is_glob {
        let g = render::render_glob_pattern(&pattern);
        (
            g.clone(),
            verify::verify_glob(seq_runs(&pattern), &positives, &negatives),
        )
    } else {
        let dialect = match for_target {
            Some(t) => t.dialect().unwrap_or(Dialect::Pcre),
            None => Dialect::parse(&cli.dialect)
                .ok_or_else(|| anyhow::anyhow!("unknown --dialect: {}", cli.dialect))?,
        };
        let pat = render::render_pattern(&pattern, dialect);
        let line = match for_target {
            Some(t) => render::wrap::wrap(t, &pat),
            None => pat.clone(),
        };
        (
            line,
            verify::verify_pattern(&pattern, &positives, &negatives),
        )
    };

    if cli.test {
        eprintln!("pe: verify {}", if report.ok { "PASS" } else { "FAIL" });
    }
    let mut code = ExitCode::SUCCESS;
    if !report.ok {
        report_failure(&report);
        code = ExitCode::from(1);
    }

    println!("{output}");

    if cli.explain {
        explain(&pattern);
    }

    Ok(code)
}

fn parse_capture(s: Option<&str>) -> anyhow::Result<Option<CaptureMode>> {
    Ok(match s {
        None => None,
        Some("anon") => Some(CaptureMode::Anon),
        Some("named") => Some(CaptureMode::Named),
        Some(other) => anyhow::bail!("--capture takes nothing or =named, got: {other}"),
    })
}

/// Borrow a pattern's runs for glob verify (alternation verifies the first
/// group; full alternation glob verify is a later step).
fn seq_runs(pattern: &Pattern) -> &[Run] {
    match pattern {
        Pattern::Seq(runs) => runs,
        Pattern::Alt(groups) => groups.first().map_or(&[], |g| g.as_slice()),
    }
}

fn report_failure(report: &verify::Report) {
    eprintln!("pe: could not fully separate positives from negatives");
    if !report.unmatched_positives.is_empty() {
        eprintln!(
            "pe:   positives not matched: {:?}",
            report.unmatched_positives
        );
    }
    if !report.admitted_negatives.is_empty() {
        eprintln!("pe:   negatives admitted: {:?}", report.admitted_negatives);
    }
}

/// §7 `--explain`: one line per run, the class chosen and why it stopped.
fn explain(pattern: &Pattern) {
    let runs = seq_runs(pattern);
    for run in runs {
        let (what, why) = match run {
            Run::Literal { text } => (format!("literal {text:?}"), "all identical"),
            Run::Enum { alts, .. } => (format!("enum {alts:?}"), "small finite set"),
            Run::Class { class, .. } => match class {
                Class::Set(cs) => (format!("set {cs:?}"), "limited to observed chars"),
                other => (format!("class {other:?}"), "reached the observed class"),
            },
        };
        eprintln!("pe: {what} — {why}");
    }
}
