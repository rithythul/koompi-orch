pub mod commit;
pub mod diff;
pub mod merge;
pub mod remote;

pub use commit::{create_commit, push_branch, stage_all, stage_files, CommitError, CommitInfo};
pub use diff::{diff_commits, diff_staged, diff_working_dir, DiffError, DiffSummary, FileDiff, FileStatus};
pub use merge::{merge_branch, rebase_branch, MergeError, MergeResult};
pub use remote::{
    create_pull_request, get_github_remote, parse_github_remote, GitHubRemote, PullRequestInfo,
    RemoteError,
};
