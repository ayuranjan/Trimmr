# trimr

`trimr` is a Rust CLI proxy for shell commands that reduces output size before it reaches an LLM.

This POC focuses on `git` and shows per-invocation token savings without any persistent database.

## Why this helps

LLMs consume context tokens quickly when command output is verbose. `trimr` reduces noise by:

- Compacting high-volume command output (`git status`, `git diff`, `git log`, etc.)
- Preserving command exit codes so automation behavior stays correct
- Printing savings for each invocation so you can see impact immediately

## Current POC scope (Phase 1)

- `git` command proxy with filtering for:
  - `status`
  - `diff`
  - `log`
  - `pull`
  - `push`
  - `commit`
  - `add`
- `cost` mode to compare raw vs filtered output side-by-side
- `debug` mode to inspect filter pipeline input and output
- Per-invocation token estimate and savings display
- No persistent storage

## Install and run

### Requirements

- Rust toolchain (`cargo`)
- `git` installed and available in PATH

### Build

```bash
cargo build
```

### Run

```bash
cargo run -- git status
cargo run -- git diff
cargo run -- cost git status
```

### Optional release build

```bash
cargo build --release
./target/release/trimr git status
```

## Usage

### Proxy git

```bash
trimr git <subcommand> [args...]
```

Examples:

```bash
trimr git status
trimr git diff
trimr git log
trimr git add src/main.rs
trimr git commit -m "message"
trimr git -C /path/to/repo status
```

### Raw vs filtered comparison

```bash
trimr cost git <subcommand> [args...]
```

Examples:

```bash
trimr cost git status
trimr cost git -C /path/to/repo status
```

### Debug filter pipeline

```bash
trimr debug git <subcommand> [args...]
```

Shows what the filter receives as input and what it produces as output. Useful when developing or debugging filter logic.

Examples:

```bash
trimr debug git status        # shows porcelain v1 input → filtered output
trimr debug git diff          # shows --stat input → filtered output
trimr debug git log           # shows compact pretty input → filtered output
trimr debug git add file.txt  # shows "no intermediate format" (filter works on raw output directly)
```

## Behavior guarantees

- Exit code propagation: `trimr` exits with the same code as `git`
- Error passthrough: failing git output is not replaced with synthetic "ok"
- Global git flags supported before subcommand (`-C`, `-c`, etc.)
- Help/version flags are forwarded to git (`trimr git --help`, `trimr git --version`)
- For `git status`, compact porcelain parsing is used only for plain status; status with user args is passed through safely

## Token savings estimate

Token count is estimated using a simple heuristic:

- `tokens ~= ceil(bytes / 4)`

This gives a fast, consistent approximation suitable for quick feedback in a CLI loop.

## Testing

```bash
cargo test
cargo clippy --all-targets --all-features
```

## Limitations in this POC

- Token estimation is heuristic, not model-specific tokenization
- Only `git` is optimized in Phase 1
- No historical analytics (per-run only)
- Filtering logic is intentionally conservative in ambiguous cases

## Project status

This repository is an early POC designed to validate command proxy behavior, correctness, and immediate token savings visibility.

See `implementationconcencept.md` for architecture and contribution details.
