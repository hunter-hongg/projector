use std::path::Path;

use anyhow::Result;
use chrono::{NaiveDateTime, TimeZone, Utc};
use git2::Repository;

pub struct GitHealth {
    pub branch: String,
    pub is_dirty: bool,
    pub unpushed_commits: u32,
    pub last_commit_date: NaiveDateTime,
}

pub fn git_health(dir: &Path) -> Result<Option<GitHealth>> {
    let repo = match Repository::open(dir) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => {
            return Ok(Some(GitHealth {
                branch: "HEAD (no commits)".to_string(),
                is_dirty: false,
                unpushed_commits: 0,
                last_commit_date: NaiveDateTime::default(),
            }));
        }
    };

    let branch = head.shorthand().unwrap_or("unknown").to_string();

    let is_dirty = {
        let statuses = repo.statuses(None)?;
        statuses.iter().any(|s| s.status() != git2::Status::CURRENT)
    };

    let unpushed_commits = count_unpublished(&repo, &head)?;

    let last_commit_date = head
        .peel_to_commit()
        .ok()
        .map(|c| {
            let time = c.time();
            let secs = time.seconds();
            Utc.timestamp_opt(secs, 0)
                .single()
                .map(|dt| dt.naive_utc())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    Ok(Some(GitHealth {
        branch,
        is_dirty,
        unpushed_commits,
        last_commit_date,
    }))
}

fn count_unpublished(repo: &Repository, head: &git2::Reference) -> Result<u32> {
    let branch_name = match head.shorthand() {
        Some(name) => name,
        None => return Ok(0),
    };

    let upstream_name = format!("refs/remotes/origin/{}", branch_name);
    let upstream = match repo.find_reference(&upstream_name) {
        Ok(r) => r,
        Err(_) => return Ok(0),
    };

    let local_commit = match head.peel_to_commit() {
        Ok(c) => c,
        Err(_) => return Ok(0),
    };

    let upstream_commit = match upstream.peel_to_commit() {
        Ok(c) => c,
        Err(_) => return Ok(0),
    };

    let merge_base = match repo.merge_base(local_commit.id(), upstream_commit.id()) {
        Ok(id) => id,
        Err(_) => return Ok(0),
    };

    let mut revwalk = repo.revwalk()?;
    revwalk.push(local_commit.id())?;
    revwalk.hide(merge_base)?;

    Ok(revwalk.count() as u32)
}

pub struct CommitStats {
    pub total: u32,
    pub last_30_days: u32,
    pub last_90_days: u32,
    pub last_year: u32,
    pub authors: u32,
}

pub fn count_commits(dir: &Path) -> Result<Option<CommitStats>> {
    let repo = match Repository::open(dir) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let mut total = 0u32;
    let mut last_30 = 0u32;
    let mut last_90 = 0u32;
    let mut last_year = 0u32;
    let mut authors = std::collections::HashSet::new();

    let now = Utc::now().naive_utc();

    let mut revwalk = match repo.revwalk() {
        Ok(w) => w,
        Err(_) => {
            return Ok(Some(CommitStats {
                total: 0,
                last_30_days: 0,
                last_90_days: 0,
                last_year: 0,
                authors: 0,
            }));
        }
    };

    if revwalk.push_head().is_err() {
        return Ok(Some(CommitStats {
            total: 0,
            last_30_days: 0,
            last_90_days: 0,
            last_year: 0,
            authors: 0,
        }));
    }

    let _ = revwalk.set_sorting(git2::Sort::TIME);

    for oid in revwalk.flatten() {
        if let Ok(commit) = repo.find_commit(oid) {
            total += 1;
            if let Some(sig) = commit.author().name() {
                authors.insert(sig.to_string());
            }
            let secs = commit.time().seconds();
            if let Some(dt) = Utc.timestamp_opt(secs, 0).single() {
                let days = (now - dt.naive_utc()).num_days();
                if days <= 30 {
                    last_30 += 1;
                }
                if days <= 90 {
                    last_90 += 1;
                }
                if days <= 365 {
                    last_year += 1;
                }
            }
        }
    }

    Ok(Some(CommitStats {
        total,
        last_30_days: last_30,
        last_90_days: last_90,
        last_year,
        authors: authors.len() as u32,
    }))
}

pub fn count_commits_since(dir: &Path, since_days: u32) -> Result<Option<u32>> {
    let repo = match Repository::open(dir) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let now = Utc::now().naive_utc();
    let mut revwalk = match repo.revwalk() {
        Ok(w) => w,
        Err(_) => return Ok(Some(0)),
    };

    if revwalk.push_head().is_err() {
        return Ok(Some(0));
    }

    let _ = revwalk.set_sorting(git2::Sort::TIME);
    let mut count = 0u32;

    for oid in revwalk.flatten() {
        if let Ok(commit) = repo.find_commit(oid) {
            let secs = commit.time().seconds();
            if let Some(dt) = Utc.timestamp_opt(secs, 0).single() {
                let days = (now - dt.naive_utc()).num_days();
                if days <= since_days as i64 {
                    count += 1;
                }
            }
        }
    }

    Ok(Some(count))
}

pub struct GitExtraHealth {
    pub stash_count: u32,
    pub untracked_files: u32,
    pub behind_upstream: u32,
}

pub fn git_extra_health(dir: &Path) -> Result<Option<GitExtraHealth>> {
    let mut repo = match Repository::open(dir) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let stash_count = {
        let mut count = 0u32;
        let _ = repo.stash_foreach(|_, _, _| {
            count += 1;
            true
        });
        count
    };

    let (untracked_files, behind_upstream) = {
        let statuses = repo.statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .recurse_untracked_dirs(true),
        ))?;

        let untracked = statuses
            .iter()
            .filter(|s| s.status() == git2::Status::WT_NEW)
            .count() as u32;

        let behind = if let Ok(head) = repo.head() {
            if let Some(name) = head.shorthand() {
                let upstream_name = format!("refs/remotes/origin/{}", name);
                if let Ok(upstream) = repo.find_reference(&upstream_name) {
                    if let (Ok(local), Ok(remote)) =
                        (head.peel_to_commit(), upstream.peel_to_commit())
                    {
                        if let Ok(merge_base) = repo.merge_base(local.id(), remote.id()) {
                            let mut revwalk = repo.revwalk().ok();
                            if let Some(ref mut w) = revwalk {
                                let _ = w.push(remote.id());
                                let _ = w.hide(merge_base);
                                w.count() as u32
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        (untracked, behind)
    };

    Ok(Some(GitExtraHealth {
        stash_count,
        untracked_files,
        behind_upstream,
    }))
}
