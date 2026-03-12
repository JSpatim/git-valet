use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{self, AsideConfig};
use crate::git_helpers::{get_git_dir, get_origin, get_work_tree, load_config, sgit};
use crate::hooks;

// ── Gitignore ────────────────────────────────────────────────────────────────

/// Adds entries to the main repo's .gitignore
fn update_gitignore(work_tree: &Path, files: &[String]) -> Result<()> {
    let gitignore_path = work_tree.join(".gitignore");

    let existing = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    let mut to_add: Vec<String> = Vec::new();
    for file in files {
        if !existing.lines().any(|line| line.trim() == file.as_str()) {
            to_add.push(file.clone());
        }
    }

    if to_add.is_empty() {
        return Ok(());
    }

    let mut content = existing;
    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content.push_str("\n# git-aside: files versioned in the aside repo\n");
    for f in &to_add {
        content.push_str(f);
        content.push('\n');
    }

    std::fs::write(&gitignore_path, content)?;
    println!("{} .gitignore updated ({} entries added)", "->".cyan(), to_add.len());
    Ok(())
}

/// Removes git-aside entries from .gitignore
fn remove_from_gitignore(work_tree: &Path, files: &[String]) -> Result<()> {
    let gitignore_path = work_tree.join(".gitignore");
    if !gitignore_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&gitignore_path)?;
    let filtered: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !files.iter().any(|f| f == trimmed)
                && trimmed != "# git-aside: files versioned in the aside repo"
        })
        .collect();

    std::fs::write(&gitignore_path, filtered.join("\n") + "\n")?;
    Ok(())
}

// ── Public commands ──────────────────────────────────────────────────────────

/// `git aside init <remote> <files...>`
pub fn init(remote: &str, files: &[String]) -> Result<()> {
    let work_tree = get_work_tree()?;
    let origin = get_origin(&work_tree)?;
    let git_dir = get_git_dir(&work_tree)?;

    let project_id = config::project_id(&origin);
    let bare_path = config::asides_dir()?.join(&project_id).join("repo.git");

    println!("{}", "Initializing aside repo...".bold());
    println!("  Project   : {}", origin.dimmed());
    println!("  Aside     : {}", remote.cyan());
    println!("  Bare repo : {}", bare_path.display().to_string().dimmed());

    // 1. Init bare repo
    std::fs::create_dir_all(&bare_path)?;
    let init_out = Command::new("git")
        .args(["init", "--bare", bare_path.to_str().unwrap()])
        .output()?;
    if !init_out.status.success() {
        anyhow::bail!("Failed to initialize bare repo");
    }

    // 2. Config
    let cfg = AsideConfig {
        work_tree: work_tree.to_str().unwrap().to_string(),
        remote: remote.to_string(),
        bare_path: bare_path.to_str().unwrap().to_string(),
        tracked: files.to_vec(),
        branch: "main".to_string(),
    };
    config::save(&cfg, &project_id)?;

    // 3. Hide untracked files from sgit status
    Command::new("git")
        .args(["--git-dir", bare_path.to_str().unwrap(), "config", "status.showUntrackedFiles", "no"])
        .output()?;

    // 4. Remote
    let remote_out = Command::new("git")
        .args(["--git-dir", bare_path.to_str().unwrap(), "remote", "add", "origin", remote])
        .output()?;
    if !remote_out.status.success() {
        // Already exists — update instead
        Command::new("git")
            .args(["--git-dir", bare_path.to_str().unwrap(), "remote", "set-url", "origin", remote])
            .output()?;
    }

    // 5. Update main repo .gitignore
    update_gitignore(&work_tree, files)?;

    // 6. Hooks
    hooks::install(&git_dir)?;
    println!("{} Git hooks installed (pre-commit, pre-push, post-merge, post-checkout)", "->".cyan());

    // 7. Initial commit if tracked files already exist
    let existing_files: Vec<&String> = files
        .iter()
        .filter(|f| work_tree.join(f).exists())
        .collect();

    if !existing_files.is_empty() {
        let add_args: Vec<&str> = std::iter::once("add")
            .chain(existing_files.iter().map(|f| f.as_str()))
            .collect();
        sgit(&add_args, &cfg)?;

        let commit_out = sgit(&["commit", "-m", "feat: init aside repo"], &cfg)?;
        if commit_out.status.success() {
            println!("{} Initial commit done", "->".cyan());

            // 8. Initial push
            let push_out = sgit(&["push", "-u", "origin", &format!("HEAD:{}", cfg.branch)], &cfg)?;
            if push_out.status.success() {
                println!("{} Initial push done", "->".cyan());
            } else {
                let err = String::from_utf8_lossy(&push_out.stderr);
                println!("{} Initial push failed (remote unreachable?): {}", "!".yellow(), err.trim());
                println!("  You can push manually with: {}", "git aside push".cyan());
            }
        }
    } else {
        println!("{} No tracked files found locally yet", "i".blue());
    }

    println!("\n{}", "Done! Aside repo initialized.".green().bold());
    println!("The following files are now managed by git-aside:");
    for f in files {
        println!("  {} {}", "-".dimmed(), f.cyan());
    }
    println!("\nYour usual git commands work as before.");

    Ok(())
}

/// `git aside status`
pub fn status() -> Result<()> {
    let cfg = load_config()?;

    println!("{}", "Aside repo status".bold());
    println!("  Remote  : {}", cfg.remote.cyan());
    println!("  Tracked :");
    for f in &cfg.tracked {
        let exists = PathBuf::from(&cfg.work_tree).join(f).exists();
        let marker = if exists { "+".green() } else { "x".red() };
        println!("    {} {}", marker, f);
    }
    println!();

    let out = sgit(&["status", "--short"], &cfg)?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim().is_empty() {
        println!("{}", "Nothing to commit — aside repo is clean.".green());
    } else {
        println!("{}", stdout);
    }

    Ok(())
}

/// `git aside sync` — add + commit + push
pub fn sync(message: &str) -> Result<()> {
    let cfg = load_config()?;

    let work_tree = PathBuf::from(&cfg.work_tree);
    let existing: Vec<&str> = cfg.tracked
        .iter()
        .filter(|f| work_tree.join(f).exists())
        .map(|f| f.as_str())
        .collect();

    if existing.is_empty() {
        println!("{}", "No tracked files found.".yellow());
        return Ok(());
    }

    let mut add_args = vec!["add"];
    add_args.extend(existing.iter());
    sgit(&add_args, &cfg)?;

    let status_out = sgit(&["status", "--porcelain"], &cfg)?;
    let has_changes = !String::from_utf8_lossy(&status_out.stdout).trim().is_empty();

    if has_changes {
        let commit_out = sgit(&["commit", "-m", message], &cfg)?;
        if !commit_out.status.success() {
            let err = String::from_utf8_lossy(&commit_out.stderr);
            println!("{} Aside commit: {}", "!".yellow(), err.trim());
        } else {
            println!("{} Aside committed", "->".cyan());
        }
    }

    push()?;
    Ok(())
}

/// `git aside push`
pub fn push() -> Result<()> {
    let cfg = load_config()?;

    let out = sgit(&["push", "origin", &format!("HEAD:{}", cfg.branch)], &cfg)?;

    if out.status.success() {
        println!("{} Aside pushed to {}", "+".green(), cfg.remote.cyan());
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("Everything up-to-date") || err.contains("up to date") {
            println!("{} Aside already up to date", "+".green());
        } else {
            println!("{} Aside push failed: {}", "!".yellow(), err.trim());
        }
    }

    Ok(())
}

/// `git aside pull`
pub fn pull() -> Result<()> {
    let cfg = load_config()?;

    let out = sgit(&["pull", "origin", &cfg.branch], &cfg)?;

    if out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains("Already up to date") || stdout.contains("up to date") {
            println!("{} Aside already up to date", "+".green());
        } else {
            println!("{} Aside updated", "+".green());
            println!("{}", stdout.trim().dimmed());
        }
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        println!("{} Aside pull failed: {}", "!".yellow(), err.trim());
    }

    Ok(())
}

/// `git aside add <files>`
pub fn add_files(files: &[String]) -> Result<()> {
    let work_tree = get_work_tree()?;
    let origin = get_origin(&work_tree)?;
    let project_id = config::project_id(&origin);

    let mut cfg = load_config()?;

    for f in files {
        if !cfg.tracked.contains(f) {
            cfg.tracked.push(f.clone());
        }
    }
    config::save(&cfg, &project_id)?;

    update_gitignore(&work_tree, files)?;

    let file_refs: Vec<&str> = files.iter().map(|f| f.as_str()).collect();
    let mut add_args = vec!["add"];
    add_args.extend(file_refs.iter());
    sgit(&add_args, &cfg)?;

    println!("{} {} file(s) added to aside", "+".green(), files.len());
    Ok(())
}

/// `git aside deinit`
pub fn deinit() -> Result<()> {
    let work_tree = get_work_tree()?;
    let origin = get_origin(&work_tree)?;
    let git_dir = get_git_dir(&work_tree)?;
    let project_id = config::project_id(&origin);

    let cfg = load_config()?;

    println!("{}", "Removing aside repo...".yellow().bold());

    hooks::uninstall(&git_dir)?;
    println!("{} Hooks removed", "->".cyan());

    remove_from_gitignore(&work_tree, &cfg.tracked)?;
    println!("{} .gitignore cleaned up", "->".cyan());

    config::remove(&project_id)?;
    println!("{} Local config removed", "->".cyan());

    println!("\n{}", "Done! Aside repo removed.".green());
    println!("{}", "Note: the remote repo is unchanged.".dimmed());

    Ok(())
}
