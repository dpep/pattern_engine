//! §7 — serialize the IR as JSON (`--json`) or NDJSON (`--ndjson`).

use serde_json::{Value, json};

use crate::ir::{Cap, Class, Run};
use crate::synth::Pattern;

fn cap_value(cap: &Option<Cap>) -> Value {
    match cap {
        None => Value::Null,
        Some(Cap { name }) => json!({ "name": name }),
    }
}

fn class_name(class: &Class) -> &'static str {
    match class {
        Class::Digit => "digit",
        Class::Lower => "lower",
        Class::Upper => "upper",
        Class::Alpha => "alpha",
        Class::Alnum => "alnum",
        Class::Word => "word",
        Class::Space => "space",
        Class::Set(_) => "set",
        Class::Any => "any",
    }
}

fn run_value(run: &Run) -> Value {
    match run {
        Run::Literal { text } => json!({ "kind": "literal", "text": text }),
        Run::Enum { alts, capture } => {
            json!({ "kind": "enum", "alts": alts, "capture": cap_value(capture) })
        }
        Run::Class {
            class,
            min,
            max,
            capture,
        } => {
            let mut obj = json!({
                "kind": "class",
                "class": class_name(class),
                "min": min,
                "max": max,
                "capture": cap_value(capture),
            });
            if let Class::Set(chars) = class {
                let set: Vec<String> = chars.iter().map(|c| c.to_string()).collect();
                obj["set"] = json!(set);
            }
            obj
        }
    }
}

fn runs_value(runs: &[Run]) -> Value {
    Value::Array(runs.iter().map(run_value).collect())
}

fn pattern_value(pattern: &Pattern) -> Value {
    match pattern {
        Pattern::Seq(runs) => runs_value(runs),
        Pattern::Alt(groups) => {
            json!({ "alt": groups.iter().map(|g| runs_value(g)).collect::<Vec<_>>() })
        }
    }
}

/// `--json`: a pretty IR document.
pub fn to_json(pattern: &Pattern) -> String {
    serde_json::to_string_pretty(&pattern_value(pattern)).unwrap_or_default()
}

/// `--ndjson`: one compact run object per line (sequences only; an alternation
/// emits one line per group's run array).
pub fn to_ndjson(pattern: &Pattern) -> String {
    let lines: Vec<String> = match pattern {
        Pattern::Seq(runs) => runs.iter().map(|r| run_value(r).to_string()).collect(),
        Pattern::Alt(groups) => groups.iter().map(|g| runs_value(g).to_string()).collect(),
    };
    lines.join("\n")
}
