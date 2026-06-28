pe — Pattern Engine
===================

**Erase what varies, keep the shape — and generalize only as far as the rejects allow.** Feed `pe` a few example strings and it emits the single tightest pattern that matches them. Add counter-examples and it widens *just* until the rejects stop it — no more. Programming by example, for regexes and globs.

```sh
$ printf 'ORD-1001\nORD-99\nORD-7\n' | pe
ORD-\d+

$ printf '2024-01-15\n2025-12-03\n' | pe
\d{4}-\d{2}-\d{2}

$ printf 'src/a.rs\nsrc/b.rs\n' | pe --for glob
src/*.rs

# rejects tighten the result — keep .log/.txt, exclude .bak
$ pe keep.txt -v reject.txt
\w+\.(log|txt)          # without -v this would be \w+\.\w+
```

Positives come from files or stdin; negatives come from `-v`. The default action is "read examples, print a pattern" — every other behavior is a flag.

## Minimal, not maximal

With only positives, `pe` climbs to the tightest class that covers what it saw and stops at the observed shape — `\d+`, not `.+`. The rejects are what tell it how far to erase: each counter-example removes a ceiling, so `pe` generalizes exactly as far as it can without admitting one.

Every emitted pattern is re-verified against all inputs before it's printed: positives must match, negatives must not. If `pe` can't separate them, it says so on stderr, prints its best effort, and exits non-zero.

## Why not write the regex yourself

- You have the examples already — the failing log lines, the filenames, the IDs. `pe` reads the shape off them so you don't hand-count digits.
- It's **deterministic**: same inputs, same output, every run. No corpus, no learning, no state, no network.
- It speaks your tool's dialect. `--for grep|sed|awk|perl|rg|glob` emits a paste-ready, correctly-escaped line; `--dialect` picks the raw flavor (PCRE, ERE, BRE, RE2, Python, JS).

## Install

```sh
brew install dpep/tools/pe     # builds from source; no runtime deps
```

Or build it yourself — `pe` needs Rust only at build time:

```sh
cargo install --path .         # or: make install
```

## Usage

```sh
pe [FILE...]                 # positive examples; stdin if no files
pe -v/--reject FILE...       # negative examples (must NOT match)
pe --for TARGET              # grep | egrep | sed | awk | perl | rg | glob
pe --dialect FLAVOR          # pcre | ere | bre | re2 | python | js  (default pcre)
pe --tight                   # keep exact observed bounds, prefer literals/enums
pe --loose                   # collapse bounds to +/*, widen classes sooner
pe -c/--capture[=named]      # wrap varying runs in (capture) groups
pe -e/--explain              # per-run: inferred class + why it stopped there
pe -l/--limit N              # emit N ranked candidates (tightest → loosest)
pe -j/--json                 # structured IR  (-J/--ndjson for one run per line)
pe --test                    # print pass/fail of verify, then the pattern
pe -h/--help · -V/--version
```

`--dialect` selects regex syntax; `--for` selects a *consumer* and implies a dialect, escaping, and wrapper. `--for glob` ignores `--dialect`. `--tight` and `--loose` are mutually exclusive; the default sits between them.

## How it works

A short pipeline, each stage swappable:

**read → tokenize → align → synthesize → verify → render.**

1. **Tokenize** each input into maximal runs by character category (digits, lower, upper, space, literal punctuation).
2. **Align** inputs that share a skeleton into columns; differing skeletons become an alternation.
3. **Synthesize** by climbing a generalization lattice per column — `literal → enum → class+bounds → class+quant → wider class → any` — stopping at the observed class with no rejects, or just below the first reject-admitting step with them.
4. **Verify** the emitted pattern against every input.
5. **Render** to the requested dialect or `--for` consumer.

See [CLAUDE.md](CLAUDE.md) for development conventions.

## License

MIT © Daniel Pepper
