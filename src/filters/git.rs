/// Parse `git status --porcelain=v1 --branch` output into compact form.
///
/// Input example:
///   "## main...origin/main [ahead 1]\n M src/main.rs\n?? foo.txt\n"
///
/// Output example:
///   "Branch: main [ahead 1]\n2 modified, 1 untracked"
pub fn filter_status(porcelain: &str) -> String {
    let mut branch_line = String::new();
    let mut added = 0u32;
    let mut modified = 0u32;
    let mut deleted = 0u32;
    let mut renamed = 0u32;
    let mut copied = 0u32;
    let mut type_changed = 0u32;
    let mut conflicted = 0u32;
    let mut untracked = 0u32;
    let mut ignored = 0u32;

    for line in porcelain.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            // "## main...origin/main [ahead 1]" or "## HEAD (no branch)"
            // Strip tracking info (everything after "...")
            let branch = if let Some(dot_pos) = rest.find("...") {
                let b = &rest[..dot_pos];
                // Check for sync status like [ahead 1] or [behind 2]
                if let Some(bracket) = rest.find('[') {
                    let end = rest.find(']').map(|i| i + 1).unwrap_or(rest.len());
                    format!("{} {}", b, &rest[bracket..end])
                } else {
                    b.to_string()
                }
            } else {
                rest.to_string()
            };
            branch_line = format!("Branch: {}", branch);
        } else if line.len() >= 2 {
            let idx = line.chars().next().unwrap_or(' ');
            let work = line.chars().nth(1).unwrap_or(' ');

            if idx == '?' && work == '?' {
                untracked += 1;
                continue;
            }
            if idx == '!' && work == '!' {
                ignored += 1;
                continue;
            }
            if is_unmerged(idx, work) {
                conflicted += 1;
                continue;
            }

            // Count by worktree status first, then index status
            match work {
                'M' => modified += 1,
                'D' => deleted += 1,
                'T' => type_changed += 1,
                _ => {}
            }
            match idx {
                'M' if work != 'M' => modified += 1,
                'A' => added += 1,
                'D' if work != 'D' => deleted += 1,
                'R' => renamed += 1,
                'C' => copied += 1,
                'T' if work != 'T' => type_changed += 1,
                _ => {}
            }
        }
    }

    let mut parts: Vec<String> = Vec::new();
    if added > 0 {
        parts.push(format!("{} added", added));
    }
    if modified > 0 {
        parts.push(format!("{} modified", modified));
    }
    if renamed > 0 {
        parts.push(format!("{} renamed", renamed));
    }
    if copied > 0 {
        parts.push(format!("{} copied", copied));
    }
    if deleted > 0 {
        parts.push(format!("{} deleted", deleted));
    }
    if type_changed > 0 {
        parts.push(format!("{} type-changed", type_changed));
    }
    if conflicted > 0 {
        parts.push(format!("{} conflicted", conflicted));
    }
    if untracked > 0 {
        parts.push(format!("{} untracked", untracked));
    }
    if ignored > 0 {
        parts.push(format!("{} ignored", ignored));
    }

    let status_line = if parts.is_empty() {
        "nothing to commit, working tree clean".to_string()
    } else {
        parts.join(", ")
    };

    if branch_line.is_empty() {
        status_line
    } else {
        format!("{}\n{}", branch_line, status_line)
    }
}

fn is_unmerged(idx: char, work: char) -> bool {
    // Porcelain-v1 unmerged states:
    // DD, AU, UD, UA, DU, AA, UU
    idx == 'U' || work == 'U' || (idx == 'A' && work == 'A') || (idx == 'D' && work == 'D')
}

/// Filter `git diff --stat --unified=1` output — cap diff body at 100 lines.
pub fn filter_diff(raw: &str) -> String {
    const MAX_LINES: usize = 100;
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= MAX_LINES {
        return raw.to_string();
    }
    let mut out = lines[..MAX_LINES].join("\n");
    out.push_str(&format!(
        "\n... [{} lines truncated]",
        lines.len() - MAX_LINES
    ));
    out
}

/// Format `git log --pretty=format:"%h %s (%cr)" -n 20` output — already compact,
/// just return as-is (the format string does the heavy lifting).
pub fn filter_log(raw: &str) -> String {
    raw.to_string()
}

/// Compact a git pull summary: keep the final "N files changed" line if present.
pub fn filter_pull(raw: &str) -> String {
    // Look for the summary line ("X files changed…" or "Already up to date.")
    for line in raw.lines() {
        if line.contains("files changed")
            || line.contains("file changed")
            || line.trim() == "Already up to date."
        {
            return line.trim().to_string();
        }
    }
    // Fallback: return last non-empty line
    raw.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("ok")
        .trim()
        .to_string()
}

/// Extract the short commit hash from `git commit` output.
pub fn filter_commit(raw: &str) -> String {
    // git commit output: "[main abc1234] message"
    for line in raw.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            if let Some(end) = t.find(']') {
                return format!("ok {}", &t[1..end]);
            }
        }
    }
    "ok".to_string()
}

/// Extract pushed branch from `git push` output (printed to stderr by git).
pub fn filter_push(raw: &str) -> String {
    for line in raw.lines() {
        let t = line.trim();
        // "Branch 'foo' set up to track…" or "Everything up-to-date"
        if t.starts_with("Branch") || t == "Everything up-to-date" {
            return t.to_string();
        }
        // " * [new branch]      main -> main"
        if t.contains("->") {
            return format!("ok {}", t);
        }
    }
    "ok".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_status_clean() {
        let input = "## main...origin/main\n";
        let out = filter_status(input);
        assert!(out.contains("Branch: main"), "got: {}", out);
        assert!(out.contains("nothing to commit"), "got: {}", out);
    }

    #[test]
    fn filter_status_modified_and_untracked() {
        let input = "## main...origin/main\n M src/main.rs\n M src/lib.rs\n?? foo.txt\n";
        let out = filter_status(input);
        assert!(out.contains("Branch: main"), "got: {}", out);
        assert!(out.contains("2 modified"), "got: {}", out);
        assert!(out.contains("1 untracked"), "got: {}", out);
    }

    #[test]
    fn filter_status_ahead() {
        let input = "## main...origin/main [ahead 1]\n";
        let out = filter_status(input);
        assert!(out.contains("[ahead 1]"), "got: {}", out);
    }

    #[test]
    fn filter_status_no_tracking() {
        let input = "## main\n M src/main.rs\n";
        let out = filter_status(input);
        assert!(out.contains("Branch: main"), "got: {}", out);
        assert!(out.contains("1 modified"), "got: {}", out);
    }

    #[test]
    fn filter_status_conflict_not_clean() {
        let input = "## main...origin/main\nUU src/main.rs\n";
        let out = filter_status(input);
        assert!(out.contains("1 conflicted"), "got: {}", out);
        assert!(!out.contains("working tree clean"), "got: {}", out);
    }

    #[test]
    fn filter_status_type_changed_not_clean() {
        let input = "## main...origin/main\n T src/main.rs\n";
        let out = filter_status(input);
        assert!(out.contains("1 type-changed"), "got: {}", out);
        assert!(!out.contains("working tree clean"), "got: {}", out);
    }

    #[test]
    fn filter_diff_truncates() {
        let many_lines: String = (0..150).map(|i| format!("line {}\n", i)).collect();
        let out = filter_diff(&many_lines);
        assert!(out.contains("truncated"), "got: {}", out);
        let line_count = out.lines().count();
        // 100 content lines + 1 truncation notice
        assert_eq!(line_count, 101, "got {} lines", line_count);
    }

    #[test]
    fn filter_diff_no_truncate_when_short() {
        let input = "diff --git a/foo.rs b/foo.rs\n+added line\n";
        let out = filter_diff(input);
        assert_eq!(out, input);
    }

    #[test]
    fn filter_pull_already_up_to_date() {
        let input = "From github.com:foo/bar\nAlready up to date.\n";
        assert_eq!(filter_pull(input), "Already up to date.");
    }

    #[test]
    fn filter_pull_files_changed() {
        let input = "remote: Counting objects...\n2 files changed, 10 insertions(+)\n";
        assert!(filter_pull(input).contains("files changed"));
    }

    #[test]
    fn filter_commit_extracts_hash() {
        let input = "[main abc1234] Add feature\n 1 file changed\n";
        let out = filter_commit(input);
        assert_eq!(out, "ok main abc1234");
    }

    #[test]
    fn filter_push_everything_up_to_date() {
        let input = "Everything up-to-date\n";
        let out = filter_push(input);
        assert_eq!(out, "Everything up-to-date");
    }

    #[test]
    fn filter_push_new_branch() {
        let input = " * [new branch]      main -> origin/main\n";
        let out = filter_push(input);
        assert!(out.contains("->"), "got: {}", out);
        assert!(out.starts_with("ok "), "got: {}", out);
    }

    #[test]
    fn filter_push_branch_tracking_line() {
        let input = "Branch 'main' set up to track remote branch 'main' from 'origin'.\n";
        let out = filter_push(input);
        assert!(out.starts_with("Branch"), "got: {}", out);
    }

    #[test]
    fn filter_push_fallback_ok() {
        let input = "some unrecognized output\n";
        let out = filter_push(input);
        assert_eq!(out, "ok");
    }
}
