/// Parse `git status --porcelain=v1 --branch` output into compact form.
///
/// Input example:
///   "## main...origin/main [ahead 1]\n M src/main.rs\n?? foo.txt\n"
///
/// Output example:
///   "Branch: main [ahead 1]\n2 modified, 1 untracked"
pub fn filter_status(porcelain: &str) -> String {
    let mut branch_line = String::new();
    let mut added: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();
    let mut deleted: Vec<String> = Vec::new();
    let mut renamed: Vec<String> = Vec::new();
    let mut copied: Vec<String> = Vec::new();
    let mut type_changed: Vec<String> = Vec::new();
    let mut conflicted: Vec<String> = Vec::new();
    let mut untracked: Vec<String> = Vec::new();
    let mut ignored: Vec<String> = Vec::new();

    for line in porcelain.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            let branch = if let Some(dot_pos) = rest.find("...") {
                let b = &rest[..dot_pos];
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
            let filename = line.get(3..).unwrap_or("").trim().to_string();

            if idx == '?' && work == '?' {
                untracked.push(filename);
                continue;
            }
            if idx == '!' && work == '!' {
                ignored.push(filename);
                continue;
            }
            if is_unmerged(idx, work) {
                conflicted.push(filename);
                continue;
            }

            match work {
                'M' => modified.push(filename.clone()),
                'D' => deleted.push(filename.clone()),
                'T' => type_changed.push(filename.clone()),
                _ => {}
            }
            match idx {
                'M' if work != 'M' => modified.push(filename.clone()),
                'A' => added.push(filename.clone()),
                'D' if work != 'D' => deleted.push(filename.clone()),
                'R' => renamed.push(filename.clone()),
                'C' => copied.push(filename.clone()),
                'T' if work != 'T' => type_changed.push(filename.clone()),
                _ => {}
            }
        }
    }

    let mut parts: Vec<String> = Vec::new();
    if !added.is_empty() {
        parts.push(format_category("added", &added));
    }
    if !modified.is_empty() {
        parts.push(format_category("modified", &modified));
    }
    if !renamed.is_empty() {
        parts.push(format_category("renamed", &renamed));
    }
    if !copied.is_empty() {
        parts.push(format_category("copied", &copied));
    }
    if !deleted.is_empty() {
        parts.push(format_category("deleted", &deleted));
    }
    if !type_changed.is_empty() {
        parts.push(format_category("type-changed", &type_changed));
    }
    if !conflicted.is_empty() {
        parts.push(format_category("conflicted", &conflicted));
    }
    if !untracked.is_empty() {
        parts.push(format_category("untracked", &untracked));
    }
    if !ignored.is_empty() {
        parts.push(format_category("ignored", &ignored));
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

/// Format a category with count and sample filename(s).
fn format_category(label: &str, files: &[String]) -> String {
    let count = files.len();
    let sample = &files[0];
    if count == 1 {
        format!("{} {} ({})", count, label, sample)
    } else {
        format!("{} {} ({}, +{} more)", count, label, sample, count - 1)
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
/// Compact `git branch` output - mark current branch, show all branches, add count.

pub fn filter_branch(raw: &str) -> String {
    let mut current = String::new();
    let mut others: Vec<String> = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(branch) = trimmed.strip_prefix("* ") {
            current = format!("* {} (current)", branch);
        } else {
            others.push(format!("  {}", trimmed));
        }
    }

    let total = if current.is_empty() { others.len() } else { others.len() + 1 };

    let mut lines: Vec<String> = Vec::new();
    if !current.is_empty() {
        lines.push(current);
    }
    lines.extend(others);
    lines.push(format!("{} branch{}", total, if total == 1 { "" } else { "es" }));

    lines.join("\n")
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
    #[test]
    fn filter_branch_single_branch() {
        let input = "* main\n";
        let out = filter_branch(input);
        assert!(out.contains("* main (current)"), "got: {}", out);
        assert!(out.contains("1 branch"), "got: {}", out);
    }

    #[test]
    fn filter_branch_multiple_branches() {
        let input = "* main\n  dev\n  feature/foo\n";
        let out = filter_branch(input);
        assert!(out.contains("* main (current)"), "got: {}", out);
        assert!(out.contains("  dev"), "got: {}", out);
        assert!(out.contains("  feature/foo"), "got: {}", out);
        assert!(out.contains("3 branches"), "got: {}", out);
    }

    #[test]
    fn filter_branch_no_branches() {
        let input = "";
        let out = filter_branch(input);
        assert!(out.contains("0 branches"), "got: {}", out);
    }
}
