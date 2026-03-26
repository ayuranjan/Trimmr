# trimr POC Implementation Concept

## 1. Purpose

This document explains what was implemented in the first `trimr` proof of concept, why design choices were made, and how contributors can safely extend it.


## 2. POC goals

Phase 1 goals:

1. Build a Rust CLI proxy around `git`
2. Reduce token-heavy output for common commands
3. Show savings on every invocation
4. Avoid persistent storage to keep POC simple

Non-goals for this phase:

- Multi-tool support beyond `git`
- Long-term analytics database
- Model-specific tokenization precision

## 3. High-level architecture

The architecture is intentionally small:

- `src/main.rs`
  - CLI parsing and command routing
- `src/commands/git.rs`
  - Git invocation parsing, process execution, dispatch, and exit-code handling
- `src/filters/git.rs`
  - Command-specific output compression logic
- `src/commands/cost.rs`
  - Raw vs filtered comparison output
- `src/commands/debug.rs`
  - Debug mode: shows filter pipeline input and output
- `src/filters/mod.rs`
  - Shared token estimate helper

Data flow for `trimr git ...`:

1. Parse CLI input
2. Parse git global flags vs subcommand vs sub-args
3. Execute raw git command
4. Decide filter or passthrough
5. Print filtered output (`FilterResult.compact_input` carries the intermediate format when one exists, e.g. porcelain v1 for status, `--stat` for diff)
6. Print per-invocation token delta
7. Exit with original git exit code

## 4. Invocation parsing model

A git command is parsed into:

- `global_args` (flags before subcommand like `-C`, `-c`)
- `subcommand` (e.g. `status`, `diff`, `log`)
- `sub_args` (remaining args)

This allows support for forms like:

- `trimr git status`
- `trimr git -C /repo status`
- `trimr git -c core.pager=cat log -n 5`

## 5. Filtering strategy in POC

### 5.1 Command-specific handling

- `status`
  - Plain `status` only: use porcelain v1 probe and compact summary
  - `status` with user args: passthrough (to avoid format-mismatch risk)
- `diff`
  - Uses `--stat --unified=1` path and truncation in filter
- `log`
  - Uses compact pretty format for default case
- `add`, `commit`, `push`, `pull`
  - Lightweight compact summaries

### 5.2 Safety over compression

The implementation now favors correctness over aggressive compaction:

- If git fails, passthrough raw output
- If a command shape can break parser assumptions, passthrough raw output
- Never emit synthetic success when git did not actually execute successfully

## 6. Correctness hardening done in this POC

These fixes were completed during the hardening pass:

1. Exit code fidelity
   - `trimr` returns git exit code exactly
2. Failure-path passthrough
   - Avoids misleading filtered `ok` responses on failed operations
3. Global option parsing
   - Supports pre-subcommand flags (`-C`, etc.)
4. Help/version forwarding
   - `trimr git --help` and `trimr git --version` are forwarded to git
5. Short-circuit global flag passthrough
   - Prevents wrong summaries when git global info flags bypass subcommand execution
6. `status` parser robustness
   - Handles conflict and type-change states before declaring clean
7. Parser mismatch prevention
   - `status` with custom args (e.g. `--porcelain=v2`) bypasses porcelain-v1 compaction

## 7. Token accounting design

Current estimator:

- `estimated_tokens = ceil(byte_len / 4)`

Why this approach:

- O(1), no allocation
- Fast enough for per-invocation CLI output
- Good enough for directional savings signal in a POC

Limitations:

- Not model-tokenizer exact
- Language/content dependent

## 8. Test strategy used

- Unit tests for filter logic and parser behavior
- Integration tests for end-to-end command behavior in temp git repos

Key regression coverage includes:

- Failure-path correctness (`add`/`commit` errors)
- Outside-repo status behavior
- Global `-C` behavior
- `--porcelain=v2` passthrough safety
- Help forwarding
- Short-circuit global flags
- Debug command routing (status/diff/log show `Filter input:`, add/push/pull/commit show `no intermediate format`)
- Debug exit code propagation

## 9. Comparison against rtk patterns

Where this POC aligns with `rtk`:

- Preserves git process semantics and exit codes
- Separates global-flag handling from subcommand dispatch
- Uses passthrough when user-provided args can invalidate compact parser assumptions

Where this POC is intentionally smaller:

- No persistent tracking database
- Fewer git subcommands and fewer advanced heuristics
- No tee/recovery output storage
- No multi-command module ecosystem yet

## 10. Contributor guide (how to extend safely)

When adding a new filtered command:

1. Add raw execution path first
2. Add compact output path
3. Keep a passthrough fallback for ambiguous/unexpected shapes
4. Preserve exit-code behavior
5. Add unit tests + integration tests for:
   - happy path
   - failure path
   - flag/format edge cases

Rule of thumb:

- If parser confidence is low, passthrough raw output

## 11. Next work roadmap

Recommended next phases:

1. Improve status summary quality
   - Include staged vs unstaged split with file samples (still compact)
2. Add deterministic tokenizer options
   - Optional model-specific token counters behind feature flags
3. Expand git command coverage
   - `show`, `branch`, `fetch`, `stash` compact modes
4. Add configurable filter levels
   - `safe` (current), `balanced`, `aggressive`
5. Add output caps and truncation controls
   - User-configurable line/token ceilings
6. Add analytics (optional Phase 2+)
   - Persist invocation stats and expose a `gain` command
7. Add shell integration docs
   - Alias and wrapper patterns for common agents
8. Harden argument parser further
   - Optional-arg global flags and edge-case parity with native git parser
9. CI and release hygiene
   - GitHub Actions for test/clippy/fmt/release checks
10. Benchmark suite
   - Measure latency overhead and token savings on real repos

## 12. Known constraints

- POC prioritizes correctness and simplicity over maximum compression
- Token estimate is approximate
- `git`-only optimization scope for now

