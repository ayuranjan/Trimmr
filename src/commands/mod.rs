pub mod cost;
pub mod debug;
pub mod git;

pub(crate) const DIVIDER: &str = "─────────────────────────────────────────";

pub(crate) fn format_git_cmd(sub: &str, args: &[String]) -> String {
    std::iter::once(sub)
        .chain(args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Merge stdout and stderr from a process output into a single string, with exit code.
pub(crate) fn combine_output(out: std::process::Output) -> (String, i32) {
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stderr.is_empty() {
        combined.push_str(&stderr);
    }
    (combined, out.status.code().unwrap_or(1))
}
