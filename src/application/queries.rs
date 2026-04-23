use crate::domain::{SortMode, Space, SpaceCounts, Task, TaskLog, ViewMode};

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
