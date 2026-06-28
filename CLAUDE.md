# pe development conventions

`pe` (Pattern Engine) reads example strings and emits the single tightest
pattern that matches them — and, given counter-examples, nothing else.
Programming by example for regexes and globs. Read [README.md](README.md) for
the product vision.

## First principles (do not drift from these)

- **Minimal, not maximal.** With only positives, climb to the tightest class
  that covers what was seen and stop at the observed shape (`\d+`, never `.+`).
  Negatives are the only thing that licenses widening, and only as far as they
  allow. When a change generalizes further than the rejects demand, it's wrong.
- **Pure function of (positives, negatives, flags).** No corpus, no learning,
  no persistence, no network, no config files. Same inputs → same output, every
  run. Stable ordering everywhere — sort enum alternates, fix column order, no
  HashMap iteration leaking into output.
- **Safe by construction.** Every emitted pattern is re-verified against all
  inputs (§verify) before printing: positives match, negatives don't. If it
  can't separate them, note it on stderr, emit best effort, exit non-zero.
- **Never emit non-regular constructs.** No backreferences, lookaround, etc. in
  generated output. We may *verify* with an engine that supports them; we never
  generate them.
- **Every command is script-friendly.** `--json`/`--ndjson` emit the IR; exit
  codes are meaningful (0 = separated cleanly, non-zero = couldn't). Keep field
  names stable.
- **ASCII-oriented by default.** Operate per-`char`, use ASCII semantics for
  `\d \w \s`, don't crash on non-ASCII. Unicode-correct classes are a
  documented v2.

## Language and toolchain

Rust, single statically-linkable binary, no runtime deps. `regex` /
`fancy-regex` / `globset` for the verify step only — never for generating
output.

This machine's Rust came via Homebrew's keg-only `rustup`, so `cargo` may not be
on `PATH`. Either add it once —

```sh
echo 'export PATH="/opt/homebrew/opt/rustup/bin:$PATH"' >> ~/.bash_profile
```

— or invoke directly: `/opt/homebrew/opt/rustup/bin/cargo`.

## Repo layout

Single binary crate (published as `pattern_engine`, binary `pe`); modules mirror
the pipeline. The IR (`ir.rs`) is the handoff between synthesis and rendering.

```text
pe/
  Cargo.toml
  src/
    main.rs        ← wire: cli → input → synth → render → print
    cli.rs         ← clap (derive) surface
    input.rs       ← read positives/negatives from stdin & files
    tokenize.rs    ← string → runs by character category
    align.rs       ← skeleton grouping + positional alignment
    synth.rs       ← the climb (lattice + reject-bounded generalization)
    ir.rs          ← Run / Class / Cap types
    render/        ← IR → string: regex (per dialect), glob, --for wrappers
    verify.rs      ← re-check the emitted pattern against all inputs
    json.rs        ← --json / --ndjson
  tests/
    cases.rs       ← table-driven gold acceptance tests
```

Keep it a single crate until there's a concrete reason to split. Simpler wins.

## Building, testing, linting

```sh
cargo build                 # dev build → target/debug/pe
cargo build --release       # optimized → target/release/pe
cargo test                  # unit + acceptance tests
cargo clippy --all-targets  # lint — keep it clean
cargo fmt                   # format — run before committing
```

Before committing: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`.

## Testing conventions

- The gold cases in `tests/cases.rs` are the contract: each asserts the exact
  rendered output for a `(positives, negatives, flags)` triple, and determinism
  is part of the assertion. Add a row when you add behavior.
- Prefer driving the library (`synthesize` + `render`) in tests over shelling
  out to the binary — faster, deterministic, no permission prompts.
- Use generic, non-identifying test data (`ORD`, `Widget`, `Foo`) — this is a
  public repo.
- Every emitted pattern in a test should pass its own verify step (the helper
  asserts this); a pattern that doesn't separate its inputs is a bug.

## Landing changes

Solo project — commit or merge directly to `main` and push; skip the PR
ceremony. Keep changes small, focused, and logically connected; change behavior
or structure, not both at once. Make sure CI is green
(`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`)
before pushing.

## Versioning / releasing

Bump the version when a change reaches users — i.e. it alters the built binary
(behavior, a flag, ranking, even `--help`/output wording). Stay below 1.0 for
now — only minor or patch bumps, never major:

- **patch** (`0.1.x`) — fixes, output/`--help` wording, internal cleanups
- **minor** (`0.x.0`) — new user-facing capability (a flag, a dialect, a target)

Repo-only docs (README, CLAUDE.md) don't bump — they don't change what `brew`
builds.

A bump is three edits, landed together:

1. `Cargo.toml` `version`
2. `Cargo.lock` — run `cargo build` so the `pattern_engine` entry updates
3. the Homebrew formula `version` in `~/code/lib/homebrew-tools/Formula/pe.rb`
   (push the tap too)
