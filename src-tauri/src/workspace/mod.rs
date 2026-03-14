pub mod conflict;
pub mod lock;
pub mod manager;
pub mod snapshot;
pub mod status;

pub use conflict::{ConflictDetector, ConflictError, ConflictWarning};
pub use lock::{LockError, LockResult, WorkspaceLock};
pub use manager::{WorktreeError, WorktreeInfo, WorktreeManager};
pub use snapshot::{SnapshotError, SnapshotInfo, SnapshotManager};
pub use status::{StatusError, StatusMachine, WorkspaceStatus};
