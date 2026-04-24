use crate::domain::{
    PendingOperationKind, SortMode, Space, SpaceCounts, SpaceId, Task, TaskId, TaskLog, ViewMode,
};
use crate::storage::TaskBucket;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListSpacesQuery {
    pub include_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowSpaceQuery {
    pub space_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListTasksQuery {
    pub space_ref: Option<String>,
    pub view: Option<ViewMode>,
    pub sort: Option<SortMode>,
    pub allow_archived_space: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowTaskQuery {
    pub task_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceSummary {
    pub space: Space,
    pub counts: SpaceCounts,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceDetails {
    pub space: Space,
    pub counts: SpaceCounts,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListEntry {
    pub task: Task,
    pub depth: usize,
    pub child_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListResult {
    pub space: Space,
    pub view: ViewMode,
    pub sort: SortMode,
    pub entries: Vec<TaskListEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDetails {
    pub task: Task,
    pub space: Space,
    pub parent: Option<Task>,
    pub children: Vec<Task>,
    pub logs: Vec<TaskLog>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationOutcome {
    pub root_task: Option<Task>,
    pub root_space: Option<Space>,
    pub affected_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub issues: Vec<DoctorIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorIssue {
    PendingOperation {
        operation_id: String,
        kind: PendingOperationKind,
    },
    MissingParent {
        task_id: TaskId,
        parent_id: TaskId,
    },
    CrossSpaceParent {
        task_id: TaskId,
        task_space_id: SpaceId,
        parent_id: TaskId,
        parent_space_id: SpaceId,
    },
    ParentCycle {
        task_id: TaskId,
    },
    BucketStatusMismatch {
        task_id: TaskId,
        bucket: TaskBucket,
        archived: bool,
    },
}
