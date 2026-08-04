use git2::Repository;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A file's status relative to HEAD, simplified to the categories the file
/// tree colors differently. Checked in this priority order (a renamed file
/// that also changed content still reads as "Renamed", matching `git
/// status`'s own compact summary).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GitFileStatus {
    New,
    Renamed,
    Modified,
}

pub struct GitController {
    repo_path: std::path::PathBuf,
}

impl GitController {
    pub fn new(repo_path: &Path) -> Self {
        Self { repo_path: repo_path.to_path_buf() }
    }

    /// Commit all modified/new .txt files with a specific message
    pub fn commit_all(&self, message: &str) -> Result<git2::Oid, String> {
        let repo = Repository::open(&self.repo_path).map_err(|e| e.to_string())?;
        let mut index = repo.index().map_err(|e| e.to_string())?;

        // Add all plain text files to stage
        index.add_all(["*.txt"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| e.to_string())?;
        index.write().map_err(|e| e.to_string())?;

        let tree_id = index.write_tree().map_err(|e| e.to_string())?;
        let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;

        let signature = repo.signature().unwrap_or_else(|_| {
            git2::Signature::now("Pneuma", "pneuma@local").unwrap()
        });

        let parent_commit = match repo.head() {
            Ok(head) => Some(head.peel_to_commit().map_err(|e| e.to_string())?),
            Err(_) => None,
        };

        let parents = match &parent_commit {
            Some(c) => vec![c],
            None => vec![],
        };

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        ).map_err(|e| e.to_string())
    }

    /// Absolute paths of tracked/untracked files that differ from HEAD —
    /// new, renamed, modified, deleted, or type-changed, staged or not —
    /// bucketed into the categories the file tree colors differently.
    /// Ignored files are excluded.
    pub fn file_statuses(&self) -> HashMap<PathBuf, GitFileStatus> {
        let mut result = HashMap::new();

        let Ok(repo) = Repository::open(&self.repo_path) else {
            return result;
        };
        let Ok(statuses) = repo.statuses(None) else {
            return result;
        };

        let new_flags = git2::Status::WT_NEW | git2::Status::INDEX_NEW;
        let renamed_flags = git2::Status::WT_RENAMED | git2::Status::INDEX_RENAMED;
        let modified_flags = git2::Status::WT_MODIFIED
            | git2::Status::WT_DELETED
            | git2::Status::WT_TYPECHANGE
            | git2::Status::INDEX_MODIFIED
            | git2::Status::INDEX_DELETED
            | git2::Status::INDEX_TYPECHANGE;

        for entry in statuses.iter() {
            let status = entry.status();
            if status.is_ignored() {
                continue;
            }

            let category = if status.intersects(new_flags) {
                GitFileStatus::New
            } else if status.intersects(renamed_flags) {
                GitFileStatus::Renamed
            } else if status.intersects(modified_flags) {
                GitFileStatus::Modified
            } else {
                continue;
            };

            if let Ok(path) = entry.path() {
                result.insert(self.repo_path.join(path), category);
            }
        }

        result
    }
}

/// Stage every change in the workspace (new, modified, and deleted files
/// alike — equivalent to `git add -A`), if it's a git repo. A no-op
/// (including on any git error) if it isn't, so callers can fire this after
/// every disk-mutating file operation without checking first.
pub fn stage_all_changes(workspace_root: &Path) {
    if !workspace_root.join(".git").is_dir() {
        return;
    }

    let Ok(repo) = Repository::open(workspace_root) else {
        return;
    };
    let Ok(mut index) = repo.index() else {
        return;
    };

    // add_all() picks up new/modified files; update_all() additionally
    // stages files that were deleted from the working tree.
    let _ = index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None);
    let _ = index.update_all(["*"].iter(), None);
    let _ = index.write();
}