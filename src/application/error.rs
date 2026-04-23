use crate::domain::{PendingOperationKind, ReferenceError, SpaceId, TaskId, ValidationError};
use crate::storage::StorageError;
use std::process::ExitCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    TerminalIo(#[from] std::io::Error),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Reference(#[from] ReferenceError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("no current space selected")]
    MissingCurrentSpace,
    #[error("space `{0}` is archived and cannot be used here")]
    ArchivedSpace(String),
    #[error("`archived` status cannot be set through this command")]
    ArchivedStatusNotAllowed,
    #[error(
        "parent task `{parent_id}` belongs to space `{parent_space_id}`, but target space is `{target_space_id}`"
    )]
    ParentSpaceMismatch {
        parent_id: TaskId,
        parent_space_id: SpaceId,
        target_space_id: SpaceId,
    },
    #[error("task `{task_id}` cannot use parent `{parent_id}` because it would create a cycle")]
    TaskParentCycle { task_id: TaskId, parent_id: TaskId },
    #[error(
        "task `{task_id}` cannot move to space `{target_space_id}` while still attached to parent `{parent_id}` in space `{parent_space_id}`"
    )]
    CrossSpaceParentMismatch {
        task_id: TaskId,
        parent_id: TaskId,
        parent_space_id: SpaceId,
        target_space_id: SpaceId,
    },
    #[error("space slug `{0}` already exists")]
    SpaceSlugConflict(String),
    #[error("task edit requires at least one change")]
    NoTaskChanges,
    #[error("task `{task_id}` must be archived before `{action}`")]
    TaskMustBeArchived {
        task_id: TaskId,
        action: &'static str,
    },
    #[error("space `{space_id}` must be archived before `{action}`")]
    SpaceMustBeArchived {
        space_id: SpaceId,
        action: &'static str,
    },
    #[error("task `{task_id}` has {child_count} child tasks; rerun with `--recursive`")]
    TaskPurgeRequiresRecursive { task_id: TaskId, child_count: usize },
    #[error("task `{task_id}` cannot be restored while ancestor `{ancestor_id}` remains archived")]
    TaskRestoreBlockedByArchivedAncestor {
        task_id: TaskId,
        ancestor_id: TaskId,
    },
    #[error(
        "task subtree rooted at `{task_id}` contains non-archived task `{offender_id}` and cannot be purged"
    )]
    TaskPurgeRequiresArchivedSubtree {
        task_id: TaskId,
        offender_id: TaskId,
    },
    #[error(
        "another multi-file operation `{operation_id}` ({kind}) is still pending; recover it first"
    )]
    PendingOperationInProgress {
        operation_id: String,
        kind: PendingOperationKind,
    },
}

impl AppError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::TerminalIo(_) => ExitCode::from(1),
            Self::Storage(_) => ExitCode::from(6),
            Self::Reference(ReferenceError::TaskNotFound(_))
            | Self::Reference(ReferenceError::SpaceNotFound(_)) => ExitCode::from(3),
            Self::Reference(ReferenceError::AmbiguousTaskReference { .. })
            | Self::Reference(ReferenceError::AmbiguousSpaceReference { .. }) => ExitCode::from(4),
            Self::Reference(ReferenceError::InvalidId(_))
            | Self::Validation(_)
            | Self::MissingCurrentSpace
            | Self::ArchivedSpace(_)
            | Self::ArchivedStatusNotAllowed
            | Self::ParentSpaceMismatch { .. }
            | Self::TaskParentCycle { .. }
            | Self::CrossSpaceParentMismatch { .. }
            | Self::SpaceSlugConflict(_)
            | Self::NoTaskChanges
            | Self::TaskMustBeArchived { .. }
            | Self::SpaceMustBeArchived { .. }
            | Self::TaskPurgeRequiresRecursive { .. }
            | Self::TaskRestoreBlockedByArchivedAncestor { .. }
            | Self::TaskPurgeRequiresArchivedSubtree { .. }
            | Self::PendingOperationInProgress { .. } => ExitCode::from(5),
        }
    }

    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::Reference(ReferenceError::AmbiguousTaskReference { .. }) => {
                Some("use the full task id instead")
            }
            Self::Reference(ReferenceError::AmbiguousSpaceReference { .. }) => {
                Some("use the full space id instead")
            }
            Self::MissingCurrentSpace => Some(
                "create a space with `todo space add <NAME>` or select one with `todo space use <SPACE_REF>`",
            ),
            Self::ArchivedSpace(_) => Some("choose an active space instead"),
            Self::ArchivedStatusNotAllowed => {
                Some("use todo, in_progress, or done; archive/restore arrives in a later stage")
            }
            Self::ParentSpaceMismatch { .. } => {
                Some("clear the parent or choose a parent task in the target space")
            }
            Self::CrossSpaceParentMismatch { .. } => {
                Some("use `--clear-parent` or set a new parent in the target space")
            }
            Self::NoTaskChanges => Some("pass at least one edit flag such as --title or --status"),
            Self::TaskMustBeArchived {
                action: "restore", ..
            } => Some(
                "use `todo task status set <TASK_REF> ...` for done tasks; restore only applies to archived tasks",
            ),
            Self::TaskMustBeArchived {
                action: "purge", ..
            } => Some("archive the task first with `todo task archive <TASK_REF>`"),
            Self::SpaceMustBeArchived {
                action: "restore", ..
            } => Some("restore only applies to archived spaces"),
            Self::SpaceMustBeArchived {
                action: "purge", ..
            } => Some("archive the space first with `todo space archive <SPACE_REF>`"),
            Self::TaskPurgeRequiresRecursive { .. } => {
                Some("rerun the command with `--recursive` to purge the whole subtree")
            }
            Self::TaskRestoreBlockedByArchivedAncestor { .. } => {
                Some("restore the archived ancestor first, or restore from the subtree root")
            }
            Self::TaskPurgeRequiresArchivedSubtree { .. } => {
                Some("only fully archived subtrees can be purged")
            }
            Self::PendingOperationInProgress { .. } => {
                Some("restart the app to auto-recover the pending operation, or run `todo doctor`")
            }
            Self::TerminalIo(_) => Some("retry in an interactive terminal session"),
            _ => None,
        }
    }
}
