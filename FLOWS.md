# trimr — Flows

## Developer Flow

1. Clone the repo
2. Run commands directly via `cargo run`:
   - `cargo run -- git status`
   - `cargo run -- cost git status`
3. Inner loop: edit → `cargo run -- <command>` → `cargo test`

No install step needed during development.

## User Flow

1. Install the binary:
   ```
   cargo install --path .
   ```
   This places `trimr` in `~/.cargo/bin/trimr`.

2. Use from anywhere:
   - `trimr git status`
   - `trimr cost git status`
   - Any other passthrough command

3. Hook wiring (future): Configure Claude Code settings to replace `git` → `trimr git`, so token counting happens transparently on every git call.

## End-to-End: `trimr git status`

```
trimr git status
```

1. **`src/main.rs`** — Clap parses args into `Commands::Git { sub: "status", args: [] }`, calls `handle_git("status", &[])`.

2. **`src/commands/git.rs` → `run_git`** — `parse_git_invocation` splits the token stream into global flags / subcommand / sub-args. Matches the `"status"` arm.

3. **Two git passes:**
   - Pass 1: `git status` — full output captured as `raw_output` (for token counting only)
   - Pass 2: `git status --porcelain=v1 --branch` — machine-readable output fed to the filter
   - If user passed extra args (e.g. `--short`), skip filtering and pass through raw output unchanged.

4. **`src/filters/git.rs` → `filter_status`** — Parses porcelain v1 line by line:
   - `## main...origin/main [ahead 1]` → `Branch: main [ahead 1]`
   - XY status codes counted into buckets: added, modified, deleted, renamed, untracked, conflicted, etc.
   - Result: `Branch: main [ahead 1]\n2 modified, 1 untracked`

5. **Back in `handle_git`:**
   - stdout ← filtered string (what Claude/the LLM reads)
   - stderr ← `[saved 42 tokens, 78%]` (human-visible, never enters LLM context)
   - Token estimate: `(byte_count + 3) / 4` — O(1), no allocation (`src/filters/mod.rs`)
