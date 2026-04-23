pub mod ids;
pub mod model;
pub mod refs;
pub mod validation;

pub use ids::{IdError, MIN_SHORT_ID_SUFFIX_LEN, SpaceId, TaskId};
pub use model::{
    AppConfig, AppState, FocusArea, PendingOperation, PendingOperationEntry, PendingOperationKind,
    SortMode, Space, SpaceCounts, SpaceListMode, SpaceState, SpaceViewMemory, StateMutation, Task,
    TaskLog, TaskStatus, TuiMemory, ViewMode, slugify,
};
pub use refs::{ReferenceError, resolve_space_ref, resolve_task_ref};
pub use validation::{ValidationError, ensure_non_empty_space_name, ensure_non_empty_title};
