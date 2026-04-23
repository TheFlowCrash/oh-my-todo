use crate::domain::{SortMode, SpaceId, TaskId, ViewMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListSpacesQuery {
    pub include_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListTasksQuery {
    pub space_id: SpaceId,
    pub view: ViewMode,
    pub sort: SortMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowTaskQuery {
    pub task_id: TaskId,
}
