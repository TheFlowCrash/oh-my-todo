use crate::domain::TaskId;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("task title cannot be empty")]
    EmptyTaskTitle,
    #[error("space name cannot be empty")]
    EmptySpaceName,
    #[error("task `{task_id}` cannot be its own parent")]
    SelfParent { task_id: TaskId },
}

pub fn ensure_non_empty_title(title: &str) -> Result<(), ValidationError> {
    if title.trim().is_empty() {
        Err(ValidationError::EmptyTaskTitle)
    } else {
        Ok(())
    }
}

pub fn ensure_non_empty_space_name(name: &str) -> Result<(), ValidationError> {
    if name.trim().is_empty() {
        Err(ValidationError::EmptySpaceName)
    } else {
        Ok(())
    }
}

pub fn ensure_not_self_parent(task_id: &TaskId, parent_id: &TaskId) -> Result<(), ValidationError> {
    if task_id == parent_id {
        Err(ValidationError::SelfParent {
            task_id: task_id.clone(),
        })
    } else {
        Ok(())
    }
}
