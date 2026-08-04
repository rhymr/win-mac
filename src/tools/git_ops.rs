use git2::{Repository, StatusOptions};
use std::path::Path;

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
}