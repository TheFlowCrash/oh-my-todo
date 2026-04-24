use crate::domain::{SpaceId, TaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveTaskDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSpaceCommand {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameSpaceCommand {
    pub space_ref: String,
    pub new_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCurrentSpaceCommand {
    pub space_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTaskCommand {
    pub title: String,
    pub space_ref: Option<String>,
    pub description: Option<String>,
    pub parent_ref: Option<String>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditTaskCommand {
    pub task_ref: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub clear_description: bool,
    pub status: Option<TaskStatus>,
    pub parent_ref: Option<String>,
    pub clear_parent: bool,
    pub space_ref: Option<String>,
}

impl EditTaskCommand {
    pub fn has_any_change(&self) -> bool {
        self.title.is_some()
            || self.description.is_some()
            || self.clear_description
            || self.status.is_some()
            || self.parent_ref.is_some()
            || self.clear_parent
            || self.space_ref.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTaskStatusCommand {
    pub task_ref: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddTaskLogCommand {
    pub task_ref: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveTaskCommand {
    pub task_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreTaskCommand {
    pub task_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgeTaskCommand {
    pub task_ref: String,
    pub recursive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveTaskCommand {
    pub task_ref: String,
    pub direction: MoveTaskDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveSpaceCommand {
    pub space_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreSpaceCommand {
    pub space_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgeSpaceCommand {
    pub space_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiLaunchCommand {
    pub space_ref: Option<String>,
    pub space_id: Option<SpaceId>,
}
