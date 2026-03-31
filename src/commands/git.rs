use anyhow::{Context, Result};
use std::process::Command;

use crate::{
    commands::combine_output,
    filters::git::{filter_branch, filter_commit, filter_diff, filter_log, filter_pull, filter_push, filter_status},
};

pub struct FilterResult {
    pub filtered_output: String,
    pub raw_output: String,
    pub exit_code: i32,
    pub compact_input: Option<String>,
}

struct ParsedGitInvocation {
    global_args: Vec<String>,
    subcommand: Option<String>,
    sub_args: Vec<String>,
}

struct GitOutput {
    combined_output: String,
    exit_code: i32,
}

impl GitOutput {
    fn success(&self) -> bool {
        self.exit_code == 0
    }

    fn from_output(out: std::process::Output) -> Self {
        let (combined_output, exit_code) = combine_output(out);
        GitOutput { combined_output, exit_code }
    }
}

fn takes_global_option_value(arg: &str) -> bool {
    matches!(
        arg,
        "-C" | "-c"
            | "--exec-path"
            | "--git-dir"
            | "--work-tree"
            | "--namespace"
            | "--super-prefix"
            | "--config-env"
            | "--attr-source"
    )
}

fn global_flags_short_circuit(global_args: &[String]) -> bool {
    global_args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "-h" | "--help" | "-v" | "--version" | "--html-path" | "--man-path" | "--info-path"
        )
    })
}

fn parse_git_invocation(sub: &str, args: &[String]) -> ParsedGitInvocation {
    let mut tokens = Vec::with_capacity(args.len() + 1);
    tokens.push(sub.to_string());
    tokens.extend(args.iter().cloned());

    let mut global_args = Vec::new();
    let mut i = 0usize;

    while i < tokens.len() {
        let token = &tokens[i];

        if token == "--" {
            global_args.push(token.clone());
            i += 1;
            break;
        }

        if !token.starts_with('-') || token == "-" {
            break;
        }

        global_args.push(token.clone());

        if takes_global_option_value(token) {
            if let Some(value) = tokens.get(i + 1) {
                global_args.push(value.clone());
                i += 1;
            } else {
                return ParsedGitInvocation {
                    global_args,
                    subcommand: None,
                    sub_args: Vec::new(),
                };
            }
        }

        i += 1;
    }

    let subcommand = tokens.get(i).cloned();
    let sub_args = if i < tokens.len() {
        tokens[i + 1..].to_vec()
    } else {
        Vec::new()
    };

    ParsedGitInvocation {
        global_args,
        subcommand,
        sub_args,
    }
}

/// Run `git [global args] [sub] [args]` and return stdout+stderr combined with exit code.
fn run_raw_git(global_args: &[String], sub: Option<&str>, args: &[String]) -> Result<GitOutput> {
    let mut cmd = Command::new("git");
    cmd.args(global_args);
    if let Some(sub) = sub {
        cmd.arg(sub);
    }
    let out = cmd
        .args(args)
        .output()
        .with_context(|| format!("failed to run `git {}`", sub.unwrap_or("<global>")))?;
    Ok(GitOutput::from_output(out))
}

/// Run git with specific args (used for the filtered variant of read-only commands).
fn run_git_args(
    global_args: &[String],
    sub: &str,
    extra_args: &[&str],
    user_args: &[String],
) -> Result<GitOutput> {
    let out = Command::new("git")
        .args(global_args)
        .arg(sub)
        .args(extra_args)
        .args(user_args)
        .output()
        .with_context(|| format!("failed to run `git {}`", sub))?;
    Ok(GitOutput::from_output(out))
}

fn passthrough_result(raw: GitOutput) -> FilterResult {
    let combined = raw.combined_output;
    FilterResult {
        filtered_output: combined.clone(),
        raw_output: combined,
        exit_code: raw.exit_code,
        compact_input: None,
    }
}

fn run_simple_filter(
    parsed: &ParsedGitInvocation,
    subcommand: &str,
    filter_fn: fn(&str) -> String,
) -> Result<FilterResult> {
    let raw = run_raw_git(&parsed.global_args, Some(subcommand), &parsed.sub_args)?;
    if !raw.success() {
        return Ok(passthrough_result(raw));
    }
    let filtered = filter_fn(&raw.combined_output);
    Ok(filtered_result(filtered, raw.combined_output, raw.exit_code, None))
}

fn filtered_result(
    filtered_output: String,
    raw_output: String,
    exit_code: i32,
    compact_input: Option<String>,
) -> FilterResult {
    FilterResult {
        filtered_output,
        raw_output,
        exit_code,
        compact_input,
    }
}

/// Dispatch `git <sub> [args]` and return a FilterResult.
pub fn run_git(sub: &str, args: &[String]) -> Result<FilterResult> {
    let parsed = parse_git_invocation(sub, args);
    let Some(subcommand) = parsed.subcommand.as_deref() else {
        let raw = run_raw_git(&parsed.global_args, None, &parsed.sub_args)?;
        return Ok(passthrough_result(raw));
    };
    if global_flags_short_circuit(&parsed.global_args) {
        let raw = run_raw_git(&parsed.global_args, Some(subcommand), &parsed.sub_args)?;
        return Ok(passthrough_result(raw));
    }

    match subcommand {
        "status" => {
            // Keep user-specified status formats as-is (e.g. --porcelain=v2, -z).
            // Compact porcelain-v1 parsing is only safe for plain `git status`.
            let raw = run_raw_git(&parsed.global_args, Some("status"), &parsed.sub_args)?;
            if !parsed.sub_args.is_empty() {
                return Ok(passthrough_result(raw));
            }

            let porcelain = run_git_args(
                &parsed.global_args,
                "status",
                &["--porcelain=v1", "--branch"],
                &[],
            )?;
            if !porcelain.success() {
                return Ok(passthrough_result(raw));
            }
            let filtered = filter_status(&porcelain.combined_output);
            Ok(filtered_result(
                filtered,
                raw.combined_output,
                porcelain.exit_code,
                Some(porcelain.combined_output),
            ))
        }
        "diff" => {
            let raw = run_raw_git(&parsed.global_args, Some("diff"), &parsed.sub_args)?;
            let stat_out = run_git_args(
                &parsed.global_args,
                "diff",
                &["--stat", "--unified=1"],
                &parsed.sub_args,
            )?;
            if !stat_out.success() {
                return Ok(passthrough_result(raw));
            }
            let filtered = filter_diff(&stat_out.combined_output);
            Ok(filtered_result(
                filtered,
                raw.combined_output,
                stat_out.exit_code,
                Some(stat_out.combined_output),
            ))
        }
        "log" => {
            let raw = run_raw_git(&parsed.global_args, Some("log"), &parsed.sub_args)?;
            if !raw.success() {
                return Ok(passthrough_result(raw));
            }
            let log_out = run_git_args(
                &parsed.global_args,
                "log",
                &["--pretty=format:%h %s (%cr)", "-n", "20"],
                &parsed.sub_args,
            )?;
            if !log_out.success() {
                return Ok(passthrough_result(raw));
            }
            let filtered = filter_log(&log_out.combined_output);
            Ok(filtered_result(
                filtered,
                raw.combined_output,
                log_out.exit_code,
                Some(log_out.combined_output),
            ))
        }
        "pull" => run_simple_filter(&parsed, "pull", filter_pull),
        "push" => run_simple_filter(&parsed, "push", filter_push),
        "commit" => run_simple_filter(&parsed, "commit", filter_commit),
        "branch" => run_simple_filter(&parsed, "branch", filter_branch),
        "add" => {
            let raw = run_raw_git(&parsed.global_args, Some("add"), &parsed.sub_args)?;
            if !raw.success() {
                return Ok(passthrough_result(raw));
            }
            Ok(filtered_result("ok".to_string(), raw.combined_output, raw.exit_code, None))
        }
        // Passthrough for anything else (checkout, branch, stash, …)
        _ => {
            let raw = run_raw_git(&parsed.global_args, Some(subcommand), &parsed.sub_args)?;
            Ok(passthrough_result(raw))
        }
    }
}

/// Run the git subcommand, print filtered output to stdout, savings to stderr.
pub fn handle_git(sub: &str, args: &[String]) -> Result<()> {
    let r = run_git(sub, args)?;

    if !r.filtered_output.is_empty() {
        print!("{}", r.filtered_output);
        if !r.filtered_output.ends_with('\n') {
            println!();
        }
    }

    if r.exit_code != 0 {
        std::process::exit(r.exit_code);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{global_flags_short_circuit, parse_git_invocation};

    #[test]
    fn parse_git_invocation_with_global_options() {
        let args = vec![
            "repo".to_string(),
            "-c".to_string(),
            "core.pager=cat".to_string(),
            "status".to_string(),
            "--short".to_string(),
        ];
        let parsed = parse_git_invocation("-C", &args);
        assert_eq!(
            parsed.global_args,
            vec!["-C", "repo", "-c", "core.pager=cat"]
        );
        assert_eq!(parsed.subcommand.as_deref(), Some("status"));
        assert_eq!(parsed.sub_args, vec!["--short"]);
    }

    #[test]
    fn parse_git_invocation_without_subcommand() {
        let parsed = parse_git_invocation("--version", &[]);
        assert_eq!(parsed.global_args, vec!["--version"]);
        assert_eq!(parsed.subcommand, None);
        assert!(parsed.sub_args.is_empty());
    }

    #[test]
    fn short_circuit_flags_detected() {
        assert!(global_flags_short_circuit(&["--version".to_string()]));
        assert!(global_flags_short_circuit(&["--html-path".to_string()]));
        assert!(!global_flags_short_circuit(&[
            "-C".to_string(),
            "repo".to_string()
        ]));
    }
}
