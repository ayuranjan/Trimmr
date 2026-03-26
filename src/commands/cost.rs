use anyhow::Result;
use colored::Colorize;

use crate::{commands::{DIVIDER, format_git_cmd, git::run_git}, filters::estimate_tokens};

/// `trimr cost git <sub> [args]` — side-by-side raw vs filtered + token counts.
pub fn handle_cost_git(sub: &str, args: &[String]) -> Result<()> {
    let r = run_git(sub, args)?;
    let cmd_tail = format_git_cmd(sub, args);

    let raw_t = estimate_tokens(r.raw_bytes);
    let out_t = estimate_tokens(r.filtered_bytes);
    let saved = raw_t as i64 - out_t as i64;
    let pct = if raw_t > 0 {
        (saved as f64 / raw_t as f64) * 100.0
    } else {
        0.0
    };

    println!("{}", format!("Command: git {}", cmd_tail).bold());
    println!("{}", DIVIDER);

    println!("{}", "Raw (git output):".dimmed());
    println!("{}", r.raw_output.trim_end());
    println!("{}", DIVIDER);

    println!("{}", "Filtered (trimr):".green().bold());
    println!("{}", r.filtered_output.trim_end());
    println!("{}", DIVIDER);

    println!(
        "Tokens:  Raw {}  →  Filtered {}  (saved {}, {:.1}%)",
        raw_t.to_string().red(),
        out_t.to_string().green(),
        saved.to_string().yellow().bold(),
        pct,
    );

    if r.exit_code != 0 {
        std::process::exit(r.exit_code);
    }

    Ok(())
}
