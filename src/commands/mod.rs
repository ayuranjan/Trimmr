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
