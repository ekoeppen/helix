use anyhow::{anyhow, Result};
use arc_swap::ArcSwap;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use crate::FileChange;

pub fn get_diff_base(_file: &Path) -> Result<Vec<u8>> {
    Err(anyhow!("get_diff_base not applicable for jj"))
}

pub fn get_current_head_name(cwd: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
    let out = Command::new("jj")
        .arg("log")
        .arg("--repository")
        .arg(cwd)
        .args([
            "--ignore-working-copy",
            "--no-pager",
            "--no-graph",
            "--color=never",
            "-r",
            "@",
            "-T",
            r#"separate(" ", change_id, surround("(", ")", bookmarks.join(", ")))"#,
        ])
        .output()?;
    if !out.status.success() {
        return Err(anyhow!("Failed to get output from jj log"));
    }
    return Ok(Arc::new(ArcSwap::from_pointee(
        String::from_utf8(out.stdout)?.into_boxed_str(),
    )));
}

pub fn for_each_changed_file(cwd: &Path, cb: impl Fn(Result<FileChange>) -> bool) -> Result<()> {
    let out = Command::new("jj")
        .arg("diff")
        .arg("--repository")
        .arg(cwd)
        .args([
            "--ignore-working-copy",
            "--color=never",
            "--no-pager",
            "--types",
            "-r",
            "@",
        ])
        .output()?;
    if !out.status.success() {
        anyhow::bail!("jj log command executed but failed");
    }
    let out = String::from_utf8(out.stdout)?;
    for line in out.lines() {
        let Some((status, path)) = line.split_once(' ') else {
            continue;
        };
        let Some(change) = status_to_change(status, path) else {
            continue;
        };
        if !cb(Ok(change)) {
            break;
        }
    }

    Ok(())
}

fn status_to_change(status: &str, path: &str) -> Option<FileChange> {
    if let rename @ Some(_) = find_rename(path) {
        return rename;
    }
    Some(match status {
        "FF" | "LL" | "CF" | "CL" | "FL" | "LF" => FileChange::Modified { path: path.into() },
        "-F" | "-L" => FileChange::Added { path: path.into() },
        "F-" | "L-" => FileChange::Deleted { path: path.into() },
        "FC" | "LC" => FileChange::Conflict { path: path.into() },
        _ => return None,
    })
}

fn find_rename(path: &str) -> Option<FileChange> {
    let (start, rest) = path.split_once('{')?;
    let (from, rest) = rest.split_once(" => ")?;
    let (to, end) = rest.split_once('}')?;
    Some(FileChange::Renamed {
        from_path: format!("{start}{from}{end}").into(),
        to_path: format!("{start}{to}{end}").into(),
    })
}
