use crate::domain::{SortMode, SpaceId, TaskId, TaskStatus, ViewMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSpaceCommand {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameSpaceCommand {
    pub space_id: SpaceId,
    pub new_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTaskCommand {
    pub title: String,
    pub space_id: SpaceId,
    pub description: Option<String>,
    pub parent_id: Option<TaskId>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTaskStatusCommand {
    pub task_id: TaskId,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCurrentSpaceCommand {
    pub space_id: SpaceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateViewCommand {
    pub view: ViewMode,
    pub sort: SortMode,
}
