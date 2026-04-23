use crate::application::error::AppError;
use crate::application::queries::{DoctorIssue, DoctorReport};
use crate::domain::{
    PendingOperation, PendingOperationEntry, PendingOperationKind, SpaceId, StateMutation, TaskId,
    TaskStatus,
};
use crate::storage::{AppRepository, TaskBucket};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use time::OffsetDateTime;
use ulid::Ulid;

#[derive(Clone)]
pub struct MaintenanceService {
    repository: Arc<dyn AppRepository>,
}

impl MaintenanceService {
    pub fn new(repository: Arc<dyn AppRepository>) -> Self {
        Self { repository }
    }

    pub fn recover_pending_operation(&self) -> Result<Option<PendingOperationKind>, AppError> {
        let state = self.repository.load_state()?;
        let Some(operation) = state.pending_operation.clone() else {
            return Ok(None);
        };

        self.apply_operation_entries(&operation)?;
        self.clear_pending_operation()?;
        Ok(Some(operation.kind))
    }

    pub fn doctor(&self) -> Result<DoctorReport, AppError> {
        let mut issues = Vec::new();
        let state = self.repository.load_state()?;
        if let Some(operation) = state.pending_operation {
            issues.push(DoctorIssue::PendingOperation {
                operation_id: operation.operation_id,
                kind: operation.kind,
            });
        }

        let records = self.repository.list_all_task_records()?;
        let tasks_by_id = records
            .iter()
            .map(|record| (record.task.id.clone(), record.task.clone()))
            .collect::<HashMap<_, _>>();

        for record in &records {
            if let Some(parent_id) = record.task.parent_id.as_ref() {
                match tasks_by_id.get(parent_id) {
                    None => issues.push(DoctorIssue::MissingParent {
                        task_id: record.task.id.clone(),
                        parent_id: parent_id.clone(),
                    }),
                    Some(parent) if parent.space_id != record.task.space_id => {
                        issues.push(DoctorIssue::CrossSpaceParent {
                            task_id: record.task.id.clone(),
                            task_space_id: record.task.space_id.clone(),
                            parent_id: parent.id.clone(),
                            parent_space_id: parent.space_id.clone(),
                        })
                    }
                    Some(_) => {}
                }
            }

            if bucket_mismatches_status(record.bucket, record.task.status) {
                issues.push(DoctorIssue::BucketStatusMismatch {
                    task_id: record.task.id.clone(),
                    bucket: record.bucket,
                    status: record.task.status,
                });
            }
        }

        let mut reported_cycles = BTreeSet::new();
        for record in &records {
            if has_parent_cycle(&record.task.id, &tasks_by_id)
                && reported_cycles.insert(record.task.id.clone())
            {
                issues.push(DoctorIssue::ParentCycle {
                    task_id: record.task.id.clone(),
                });
            }
        }

        Ok(DoctorReport { issues })
    }

    pub(crate) fn execute_operation(
        &self,
        kind: PendingOperationKind,
        entries: Vec<PendingOperationEntry>,
    ) -> Result<(), AppError> {
        let operation = PendingOperation {
            operation_id: format!("op_{}", Ulid::new()),
            kind,
            created_at: OffsetDateTime::now_utc(),
            entries,
        };

        let mut state = self.repository.load_state()?;
        if let Some(existing) = state.pending_operation {
            return Err(AppError::PendingOperationInProgress {
                operation_id: existing.operation_id,
                kind: existing.kind,
            });
        }

        state.pending_operation = Some(operation.clone());
        self.repository.save_state(&state)?;
        self.apply_operation_entries(&operation)?;
        self.clear_pending_operation()
    }

    fn apply_operation_entries(&self, operation: &PendingOperation) -> Result<(), AppError> {
        for entry in &operation.entries {
            match entry {
                PendingOperationEntry::TaskUpsert(task) => self.repository.save_task(task)?,
                PendingOperationEntry::TaskDelete { task_id } => {
                    self.repository.delete_task(task_id)?
                }
                PendingOperationEntry::SpaceUpsert(space) => self.repository.save_space(space)?,
                PendingOperationEntry::SpaceDelete { space_id } => {
                    self.repository.delete_space(space_id)?
                }
                PendingOperationEntry::StateUpdate(mutation) => {
                    self.apply_state_mutation(mutation)?
                }
            }
        }

        Ok(())
    }

    fn apply_state_mutation(&self, mutation: &StateMutation) -> Result<(), AppError> {
        let mut state = self.repository.load_state()?;
        state.current_space_id = mutation.current_space_id.clone();
        for space_id in &mutation.cleared_space_memory_ids {
            state.tui_memory.spaces.remove(space_id);
        }
        self.repository.save_state(&state)?;
        Ok(())
    }

    fn clear_pending_operation(&self) -> Result<(), AppError> {
        let mut state = self.repository.load_state()?;
        state.pending_operation = None;
        self.repository.save_state(&state)?;
        Ok(())
    }
}

fn bucket_mismatches_status(bucket: TaskBucket, status: TaskStatus) -> bool {
    matches!(bucket, TaskBucket::Todo) && status.is_archived()
        || matches!(bucket, TaskBucket::Archive) && !status.is_archived()
}

fn has_parent_cycle(task_id: &TaskId, tasks_by_id: &HashMap<TaskId, crate::domain::Task>) -> bool {
    let mut seen = HashSet::<TaskId>::new();
    let mut current_id = Some(task_id.clone());

    while let Some(candidate_id) = current_id {
        if !seen.insert(candidate_id.clone()) {
            return true;
        }

        let Some(task) = tasks_by_id.get(&candidate_id) else {
            return false;
        };

        current_id = task.parent_id.clone();
    }

    false
}

pub(crate) fn next_active_space_id(
    current_space_id: Option<&SpaceId>,
    spaces: &[crate::domain::Space],
    excluding: Option<&SpaceId>,
) -> Option<SpaceId> {
    let mut active_spaces = spaces
        .iter()
        .filter(|space| space.state.is_active())
        .filter(|space| excluding != Some(&space.id))
        .collect::<Vec<_>>();

    active_spaces.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.created_at.cmp(&right.created_at))
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });

    if active_spaces.is_empty() {
        return None;
    }

    if let Some(current_space_id) = current_space_id {
        if let Some(index) = active_spaces
            .iter()
            .position(|space| &space.id == current_space_id)
        {
            if let Some(next_space) = active_spaces.get(index + 1) {
                return Some(next_space.id.clone());
            }
        }
    }

    active_spaces.first().map(|space| space.id.clone())
}
