# trimr — Supported Commands

## git

Invoke via `trimr git <subcommand>` or `trimr cost git <subcommand>`.

| Subcommand | Filtering applied |
|------------|-------------------|
| `status`   | Compact summary: branch name, sync state, counts of added/modified/deleted/untracked/etc. |
| `diff`     | `--stat --unified=1` format, capped at 100 lines with truncation notice |
| `log`      | Short format: `<hash> <subject> (<relative time>)`, last 20 commits |
| `add`      | Silenced — outputs `ok` on success |
| `commit`   | Extracts branch + short hash: `ok <branch> <hash>` |
| `push`     | Extracts branch tracking line or `Everything up-to-date` |
| `pull`     | Extracts `N files changed` summary or `Already up to date.` |
| *(other)*  | Passed through unmodified |

All other git subcommands (e.g. `checkout`, `branch`, `stash`, `rebase`) pass through to git unchanged.
