use crate::domain::{SpaceId, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use time::OffsetDateTime;

pub const APP_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Todo,
    InProgress,
    Done,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpaceState {
    #[default]
    Active,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    #[default]
    Todo,
    Archive,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortMode {
    Created,
    #[default]
    Updated,
    Status,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FocusArea {
    Spaces,
    #[default]
    TaskTree,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpaceListMode {
    #[default]
    Active,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpaceCounts {
    pub todo_tasks: usize,
    pub archived_tasks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLog {
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub space_id: SpaceId,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub archived: bool,
    pub parent_id: Option<TaskId>,
    pub sort_order: i64,
    pub logs: Vec<TaskLog>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Space {
    pub id: SpaceId,
    pub name: String,
    pub slug: String,
    pub state: SpaceState,
    pub sort_order: i64,
    pub counts: SpaceCounts,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub schema_version: u32,
    pub default_view: ViewMode,
    pub default_sort: SortMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SpaceViewMemory {
    pub selected_task_id: Option<TaskId>,
    pub expanded_task_ids: Vec<TaskId>,
    pub task_tree_scroll: usize,
    pub details_scroll: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingOperationKind {
    TaskArchive,
    TaskRestore,
    TaskPurge,
    SpaceArchive,
    SpaceRestore,
    SpacePurge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StateMutation {
    pub current_space_id: Option<SpaceId>,
    pub cleared_space_memory_ids: Vec<SpaceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingOperationEntry {
    TaskUpsert(Task),
    TaskDelete { task_id: TaskId },
    SpaceUpsert(Space),
    SpaceDelete { space_id: SpaceId },
    StateUpdate(StateMutation),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingOperation {
    pub operation_id: String,
    pub kind: PendingOperationKind,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub entries: Vec<PendingOperationEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TuiMemory {
    pub focus_area: FocusArea,
    pub spaces_cursor: usize,
    pub selected_space_id: Option<SpaceId>,
    pub space_list_mode: SpaceListMode,
    pub task_filter: String,
    pub spaces: BTreeMap<SpaceId, SpaceViewMemory>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub current_space_id: Option<SpaceId>,
    pub current_view: ViewMode,
    pub current_sort: SortMode,
    pub tui_memory: TuiMemory,
    pub pending_operation: Option<PendingOperation>,
}

impl TaskStatus {
    pub fn is_open(self) -> bool {
        matches!(self, Self::Todo | Self::InProgress)
    }

    pub fn is_finished(self) -> bool {
        matches!(self, Self::Done | Self::Close)
    }
}

impl SpaceState {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    pub fn is_archived(self) -> bool {
        matches!(self, Self::Archived)
    }
}

impl SpaceListMode {
    pub fn includes_archived(self) -> bool {
        matches!(self, Self::All)
    }
}

impl PendingOperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TaskArchive => "task_archive",
            Self::TaskRestore => "task_restore",
            Self::TaskPurge => "task_purge",
            Self::SpaceArchive => "space_archive",
            Self::SpaceRestore => "space_restore",
            Self::SpacePurge => "space_purge",
        }
    }
}

impl fmt::Display for PendingOperationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

impl Task {
    pub fn new(title: impl Into<String>, space_id: SpaceId, sort_order: i64) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: TaskId::new(),
            space_id,
            title: title.into(),
            description: None,
            status: TaskStatus::Todo,
            archived: false,
            parent_id: None,
            sort_order,
            logs: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_visible_in_view(&self, view: ViewMode) -> bool {
        match view {
            ViewMode::Todo => !self.archived,
            ViewMode::Archive => self.archived,
            ViewMode::All => true,
        }
    }

    pub fn storage_bucket(&self) -> &'static str {
        if self.archived { "archive" } else { "todo" }
    }

    pub fn touch(&mut self, now: OffsetDateTime) {
        self.updated_at = now;
    }
}

impl Space {
    pub fn new(name: impl Into<String>, sort_order: i64) -> Self {
        let now = OffsetDateTime::now_utc();
        let name = name.into();
        Self {
            id: SpaceId::new(),
            slug: slugify(&name),
            name,
            state: SpaceState::Active,
            sort_order,
            counts: SpaceCounts::default(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn rename(&mut self, name: impl Into<String>, now: OffsetDateTime) {
        self.name = name.into();
        self.updated_at = now;
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: APP_SCHEMA_VERSION,
            default_view: ViewMode::Todo,
            default_sort: SortMode::Updated,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_space_id: None,
            current_view: ViewMode::Todo,
            current_sort: SortMode::Updated,
            tui_memory: TuiMemory::default(),
            pending_operation: None,
        }
    }
}

pub fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator && !slug.is_empty() {
            slug.push('_');
            previous_was_separator = true;
        }
    }

    while slug.ends_with('_') {
        slug.pop();
    }

    if slug.is_empty() {
        "space".to_owned()
    } else {
        slug
    }
}
