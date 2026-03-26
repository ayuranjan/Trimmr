use anyhow::Result;
use colored::Colorize;

use crate::commands::{DIVIDER, format_git_cmd, git::run_git};

/// `trimr debug git <sub> [args]` — show intermediate porcelain/compact input and filtered output.
pub fn handle_debug_git(sub: &str, args: &[String]) -> Result<()> {
    let r = run_git(sub, args)?;
    let cmd_tail = format_git_cmd(sub, args);

    println!("{}", format!("Command: git {}", cmd_tail).bold());
    println!("{}", DIVIDER);

    match &r.compact_input {
        Some(compact) => {
            println!("{}", "Filter input:".dimmed());
            println!("{}", compact.trim_end());
        }
        None => {
            println!(
                "{}",
                "no intermediate format — filter works directly on raw output".dimmed()
            );
            println!("{}", r.raw_output.trim_end());
        }
    }

    println!("{}", DIVIDER);
    println!("{}", "Filtered output:".green().bold());
    println!("{}", r.filtered_output.trim_end());

    if r.exit_code != 0 {
        std::process::exit(r.exit_code);
    }

    Ok(())
}
