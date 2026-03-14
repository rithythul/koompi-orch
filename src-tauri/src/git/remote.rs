//! PR creation and remote operations via octocrab (GitHub API).
//!
//! Extracts owner/repo from git remote URL, then uses octocrab to
//! create pull requests and fetch CI status.

use git2::Repository;
use octocrab::Octocrab;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RemoteError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("github API error: {0}")]
    GitHub(#[from] octocrab::Error),
    #[error("remote not found: {0}")]
    RemoteNotFound(String),
    #[error("cannot parse remote URL: {0}")]
    InvalidRemoteUrl(String),
    #[error("no GitHub token configured")]
    NoToken,
}

/// Info about a created pull request.
#[derive(Debug, Clone)]
pub struct PullRequestInfo {
    pub number: u64,
    pub html_url: String,
    pub title: String,
    pub head_branch: String,
    pub base_branch: String,
}

/// Parsed GitHub remote: owner and repo name.
#[derive(Debug, Clone)]
pub struct GitHubRemote {
    pub owner: String,
    pub repo: String,
}

/// Parse a GitHub remote URL into owner/repo.
///
/// Supports:
/// - `https://github.com/owner/repo.git`
/// - `https://github.com/owner/repo`
/// - `git@github.com:owner/repo.git`
/// - `ssh://git@github.com/owner/repo.git`
pub fn parse_github_remote(url: &str) -> Result<GitHubRemote, RemoteError> {
    // SSH format: git@github.com:owner/repo.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let cleaned = rest.trim_end_matches(".git");
        let parts: Vec<&str> = cleaned.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok(GitHubRemote {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
            });
        }
    }

    // HTTPS format: https://github.com/owner/repo[.git]
    // SSH URL format: ssh://git@github.com/owner/repo[.git]
    if url.contains("github.com") {
        let after_host = url
            .split("github.com")
            .nth(1)
            .ok_or_else(|| RemoteError::InvalidRemoteUrl(url.to_string()))?;
        // Remove leading / or :
        let path = after_host
            .trim_start_matches('/')
            .trim_start_matches(':');
        let cleaned = path.trim_end_matches(".git");
        let parts: Vec<&str> = cleaned.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok(GitHubRemote {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
            });
        }
    }

    Err(RemoteError::InvalidRemoteUrl(url.to_string()))
}

/// Extract the GitHub remote info from a repository's remote.
pub fn get_github_remote(
    worktree_path: &Path,
    remote_name: &str,
) -> Result<GitHubRemote, RemoteError> {
    let repo = Repository::open(worktree_path)?;
    let remote = repo
        .find_remote(remote_name)
        .map_err(|_| RemoteError::RemoteNotFound(remote_name.to_string()))?;
    let url = remote
        .url()
        .ok_or_else(|| RemoteError::InvalidRemoteUrl("no URL".to_string()))?;
    parse_github_remote(url)
}

/// Create a pull request on GitHub.
///
/// Requires a GitHub personal access token. The `head` branch must already
/// be pushed to the remote.
pub async fn create_pull_request(
    token: &str,
    owner: &str,
    repo: &str,
    title: &str,
    body: &str,
    head_branch: &str,
    base_branch: &str,
) -> Result<PullRequestInfo, RemoteError> {
    let octocrab = Octocrab::builder()
        .personal_token(token.to_string())
        .build()?;

    let pr = octocrab
        .pulls(owner, repo)
        .create(title, head_branch, base_branch)
        .body(body)
        .send()
        .await?;

    Ok(PullRequestInfo {
        number: pr.number,
        html_url: pr
            .html_url
            .map(|u| u.to_string())
            .unwrap_or_default(),
        title: pr.title.unwrap_or_default().to_string(),
        head_branch: head_branch.to_string(),
        base_branch: base_branch.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use tempfile::TempDir;

    #[test]
    fn test_parse_github_https_url() {
        let remote = parse_github_remote("https://github.com/koompi/orch.git").unwrap();
        assert_eq!(remote.owner, "koompi");
        assert_eq!(remote.repo, "orch");
    }

    #[test]
    fn test_parse_github_https_no_git_suffix() {
        let remote = parse_github_remote("https://github.com/koompi/orch").unwrap();
        assert_eq!(remote.owner, "koompi");
        assert_eq!(remote.repo, "orch");
    }

    #[test]
    fn test_parse_github_ssh_url() {
        let remote = parse_github_remote("git@github.com:koompi/orch.git").unwrap();
        assert_eq!(remote.owner, "koompi");
        assert_eq!(remote.repo, "orch");
    }

    #[test]
    fn test_parse_github_ssh_protocol_url() {
        let remote =
            parse_github_remote("ssh://git@github.com/koompi/orch.git").unwrap();
        assert_eq!(remote.owner, "koompi");
        assert_eq!(remote.repo, "orch");
    }

    #[test]
    fn test_parse_invalid_url() {
        let result = parse_github_remote("https://gitlab.com/user/repo.git");
        assert!(matches!(result, Err(RemoteError::InvalidRemoteUrl(_))));
    }

    #[test]
    fn test_get_github_remote_no_remote() {
        let dir = TempDir::new().unwrap();
        Repository::init(dir.path()).unwrap();
        let result = get_github_remote(dir.path(), "origin");
        assert!(matches!(result, Err(RemoteError::RemoteNotFound(_))));
    }

    #[test]
    fn test_get_github_remote_with_remote() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        repo.remote("origin", "https://github.com/koompi/orch.git")
            .unwrap();

        let remote = get_github_remote(dir.path(), "origin").unwrap();
        assert_eq!(remote.owner, "koompi");
        assert_eq!(remote.repo, "orch");
    }
}
