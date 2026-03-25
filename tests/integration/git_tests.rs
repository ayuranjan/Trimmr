use assert_cmd::Command;
use predicates::{prelude::PredicateBooleanExt, str::contains};
use std::fs;
use tempfile::TempDir;

fn make_git_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    // Init repo with an initial commit so HEAD exists
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();

    fs::write(path.join("README.md"), "# test\n").unwrap();
    std::process::Command::new("git")
        .args(["add", "README.md"])
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(path)
        .output()
        .unwrap();

    dir
}

#[test]
fn git_status_clean_repo() {
    let dir = make_git_repo();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "status"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("Branch:"))
        .stdout(contains("nothing to commit"));
}

#[test]
fn git_status_modified_file() {
    let dir = make_git_repo();
    fs::write(dir.path().join("README.md"), "# modified\n").unwrap();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "status"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("modified"));
}

#[test]
fn git_status_untracked() {
    let dir = make_git_repo();
    fs::write(dir.path().join("new.txt"), "new file\n").unwrap();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "status"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("untracked"));
}

#[test]
fn git_log_compact() {
    let dir = make_git_repo();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "log"])
        .current_dir(dir.path())
        .assert()
        .success()
        // compact format: short-hash + subject + relative time
        .stdout(contains("init"));
}

#[test]
fn git_add_prints_ok() {
    let dir = make_git_repo();
    fs::write(dir.path().join("new.txt"), "content\n").unwrap();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "add", "new.txt"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("ok"));
}

#[test]
fn git_add_missing_file_fails() {
    let dir = make_git_repo();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "add", "does-not-exist.txt"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stdout(contains("pathspec"))
        .stdout(contains("ok").not());
}

#[test]
fn git_commit_prints_hash() {
    let dir = make_git_repo();
    fs::write(dir.path().join("new.txt"), "content\n").unwrap();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "add", "new.txt"])
        .current_dir(dir.path())
        .assert()
        .success();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "commit", "-m", "add new.txt"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("ok"));
}

#[test]
fn git_commit_without_changes_fails() {
    let dir = make_git_repo();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "commit", "-m", "no changes"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stdout(contains("nothing to commit").or(contains("no changes added")));
}

#[test]
fn git_status_outside_repo_fails() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "status"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stdout(contains("not a git repository"));
}

#[test]
fn git_passthrough_reports_savings_even_when_zero() {
    let dir = make_git_repo();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "rev-parse", "--is-inside-work-tree"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(contains("[saved 0 tokens, 0%]"));
}

#[test]
fn git_global_help_is_forwarded() {
    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "--help"])
        .assert()
        .success()
        .stdout(contains("usage: git"))
        .stdout(contains("Usage: trimr git").not());
}

#[test]
fn git_subcommand_help_is_forwarded() {
    let dir = make_git_repo();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "status", "-h"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stdout(contains("usage: git status"))
        .stdout(contains("Branch:").not());
}

#[test]
fn git_short_circuit_global_flags_passthrough() {
    let dir = make_git_repo();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "--version", "commit"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("git version"))
        .stdout(contains("ok").not());
}

#[test]
fn git_status_with_c_global_option_is_filtered() {
    let repo = make_git_repo();
    let outside = TempDir::new().unwrap();
    let repo_path = repo.path().to_str().unwrap();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "-C", repo_path, "status"])
        .current_dir(outside.path())
        .assert()
        .success()
        .stdout(contains("Branch:"))
        .stdout(contains("nothing to commit"));
}

#[test]
fn git_status_with_porcelain_v2_is_passthrough() {
    let dir = make_git_repo();
    fs::write(dir.path().join("README.md"), "# modified\n").unwrap();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["git", "status", "--porcelain=v2"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("README.md"))
        .stdout(contains("nothing to commit").not());
}

#[test]
fn cost_git_status_with_c_global_option_shows_comparison() {
    let repo = make_git_repo();
    let outside = TempDir::new().unwrap();
    let repo_path = repo.path().to_str().unwrap();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["cost", "git", "-C", repo_path, "status"])
        .current_dir(outside.path())
        .assert()
        .success()
        .stdout(contains("Command: git -C"))
        .stdout(contains("Filtered"))
        .stdout(contains("Tokens:"));
}

#[test]
fn cost_git_help_is_forwarded() {
    Command::cargo_bin("trimr")
        .unwrap()
        .args(["cost", "git", "--help"])
        .assert()
        .success()
        .stdout(contains("Command: git --help"))
        .stdout(contains("usage: git"));
}

#[test]
fn cost_git_status_shows_comparison() {
    let dir = make_git_repo();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["cost", "git", "status"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("Command: git status"))
        .stdout(contains("Raw"))
        .stdout(contains("Filtered"))
        .stdout(contains("Tokens:"));
}

#[test]
fn cost_git_diff_shows_comparison() {
    let dir = make_git_repo();
    fs::write(dir.path().join("README.md"), "# modified\nextra line\n").unwrap();

    Command::cargo_bin("trimr")
        .unwrap()
        .args(["cost", "git", "diff"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("Tokens:"));
}
