use crate::application::commands::{
    AddTaskLogCommand, ArchiveTaskCommand, CreateTaskCommand, EditTaskCommand, MoveTaskCommand,
    MoveTaskDirection, PurgeTaskCommand, RestoreTaskCommand, UpdateTaskStatusCommand,
};
use crate::application::error::AppError;
use crate::application::maintenance_service::MaintenanceService;
use crate::application::queries::{
    ListTasksQuery, OperationOutcome, ShowTaskQuery, TaskDetails, TaskListResult,
};
use crate::application::task_query::{build_task_list, sort_tasks_in_place};
use crate::domain::{
    PendingOperationEntry, PendingOperationKind, SortMode, Space, Task, TaskId, TaskLog,
    TaskStatus, ViewMode, ensure_non_empty_title, resolve_task_ref,
};
use crate::storage::AppRepository;
use std::collections::HashSet;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct TaskService {
    repository: Arc<dyn AppRepository>,
    maintenance_service: MaintenanceService,
}

impl TaskService {
    pub fn new(repository: Arc<dyn AppRepository>) -> Self {
        let maintenance_service = MaintenanceService::new(repository.clone());
        Self {
            repository,
            maintenance_service,
        }
    }

    pub fn create_task(&self, command: CreateTaskCommand) -> Result<Task, AppError> {
        ensure_non_empty_title(&command.title)?;
        ensure_active_status(command.status)?;

        let all_tasks = self.repository.list_all_tasks()?;
        let parent_task = if let Some(parent_ref) = command.parent_ref.as_deref() {
            Some(self.resolve_task_from(&all_tasks, parent_ref)?)
        } else {
            None
        };

        let target_space = match parent_task.as_ref() {
            Some(parent) => {
                let space = self.repository.load_space(&parent.space_id)?;
                ensure_active_space(&space)?;

                if let Some(space_ref) = command.space_ref.as_deref() {
                    let requested_space = self.resolve_space(space_ref, true)?;
                    if requested_space.id != parent.space_id {
                        return Err(AppError::ParentSpaceMismatch {
                            parent_id: parent.id.clone(),
                            parent_space_id: parent.space_id.clone(),
                            target_space_id: requested_space.id,
                        });
                    }
                }

                space
            }
            None => self.resolve_effective_space(command.space_ref.as_deref(), true)?,
        };

        let mut task = Task::new(
            command.title,
            target_space.id.clone(),
            next_sort_order(
                &all_tasks,
                &target_space.id,
                parent_task.as_ref().map(|task| &task.id),
                &HashSet::new(),
            ),
        );
        task.description = command.description;
        task.parent_id = parent_task.as_ref().map(|task| task.id.clone());
        task.status = command.status;

        self.repository.save_task(&task)?;
        Ok(task)
    }

    pub fn list_tasks(&self, query: ListTasksQuery) -> Result<TaskListResult, AppError> {
        let state = self.repository.load_state()?;
        let space =
            self.resolve_effective_space(query.space_ref.as_deref(), !query.allow_archived_space)?;
        let view = query.view.unwrap_or(ViewMode::Todo);
        let sort = query.sort.unwrap_or(state.current_sort);
        let tasks = self.repository.list_tasks_in_space(&space.id)?;

        Ok(build_task_list(space, tasks, view, sort))
    }

    pub fn show_task(&self, query: ShowTaskQuery) -> Result<TaskDetails, AppError> {
        let all_tasks = self.repository.list_all_tasks()?;
        let task = self.resolve_task_from(&all_tasks, &query.task_ref)?;
        let space = self.repository.load_space(&task.space_id)?;

        let parent = task
            .parent_id
            .as_ref()
            .and_then(|parent_id| {
                all_tasks
                    .iter()
                    .find(|candidate| &candidate.id == parent_id)
            })
            .cloned();

        let mut children = all_tasks
            .iter()
            .filter(|candidate| candidate.parent_id.as_ref() == Some(&task.id))
            .cloned()
            .collect::<Vec<_>>();
        sort_tasks_in_place(&mut children, SortMode::Manual);

        let mut logs = task.logs.clone();
        logs.sort_by(|left, right| right.at.cmp(&left.at));

        Ok(TaskDetails {
            task,
            space,
            parent,
            children,
            logs,
        })
    }

    pub fn edit_task(&self, command: EditTaskCommand) -> Result<Task, AppError> {
        if !command.has_any_change() {
            return Err(AppError::NoTaskChanges);
        }

        if let Some(title) = command.title.as_deref() {
            ensure_non_empty_title(title)?;
        }

        if let Some(status) = command.status {
            ensure_active_status(status)?;
        }

        let all_tasks = self.repository.list_all_tasks()?;
        let current_task = self.resolve_task_from(&all_tasks, &command.task_ref)?;
        let subtree_ids = collect_subtree_ids(&all_tasks, &current_task.id);
        let parent_change_requested = command.clear_parent || command.parent_ref.is_some();

        let desired_parent = if command.clear_parent {
            None
        } else if let Some(parent_ref) = command.parent_ref.as_deref() {
            let parent = self.resolve_task_from(&all_tasks, parent_ref)?;
            if subtree_ids.contains(&parent.id) {
                return Err(AppError::TaskParentCycle {
                    task_id: current_task.id.clone(),
                    parent_id: parent.id,
                });
            }
            Some(parent)
        } else {
            current_task.parent_id.as_ref().and_then(|parent_id| {
                all_tasks
                    .iter()
                    .find(|candidate| &candidate.id == parent_id)
                    .cloned()
            })
        };

        let desired_space = if let Some(parent) = desired_parent.as_ref() {
            let space = self.repository.load_space(&parent.space_id)?;
            ensure_active_space(&space)?;

            if let Some(space_ref) = command.space_ref.as_deref() {
                let requested_space = self.resolve_space(space_ref, true)?;
                if requested_space.id != parent.space_id {
                    return Err(AppError::ParentSpaceMismatch {
                        parent_id: parent.id.clone(),
                        parent_space_id: parent.space_id.clone(),
                        target_space_id: requested_space.id,
                    });
                }
            }

            space
        } else if let Some(space_ref) = command.space_ref.as_deref() {
            self.resolve_space(space_ref, true)?
        } else {
            self.repository.load_space(&current_task.space_id)?
        };

        if command.space_ref.is_some() && !parent_change_requested {
            if let Some(parent) = current_task.parent_id.as_ref().and_then(|parent_id| {
                all_tasks
                    .iter()
                    .find(|candidate| &candidate.id == parent_id)
            }) {
                if parent.space_id != desired_space.id {
                    return Err(AppError::CrossSpaceParentMismatch {
                        task_id: current_task.id.clone(),
                        parent_id: parent.id.clone(),
                        parent_space_id: parent.space_id.clone(),
                        target_space_id: desired_space.id.clone(),
                    });
                }
            }
        }

        let mut updated_tasks = all_tasks
            .iter()
            .filter(|task| subtree_ids.contains(&task.id))
            .cloned()
            .collect::<Vec<_>>();
        let now = OffsetDateTime::now_utc();
        let parent_changed =
            desired_parent.as_ref().map(|task| task.id.clone()) != current_task.parent_id;
        let space_changed = desired_space.id != current_task.space_id;

        for task in &mut updated_tasks {
            if space_changed {
                task.space_id = desired_space.id.clone();
                task.touch(now);
            }
        }

        let root = updated_tasks
            .iter_mut()
            .find(|task| task.id == current_task.id)
            .expect("edited task must be present in subtree");

        if let Some(title) = command.title {
            root.title = title;
        }
        if let Some(description) = command.description {
            root.description = Some(description);
        }
        if command.clear_description {
            root.description = None;
        }
        if let Some(status) = command.status {
            root.status = status;
        }
        if parent_change_requested {
            root.parent_id = desired_parent.as_ref().map(|task| task.id.clone());
        }
        if parent_changed || space_changed {
            root.sort_order = next_sort_order(
                &all_tasks,
                &desired_space.id,
                desired_parent.as_ref().map(|task| &task.id),
                &subtree_ids,
            );
        }
        root.space_id = desired_space.id.clone();
        root.touch(now);
        let result = root.clone();

        for task in &updated_tasks {
            self.repository.save_task(task)?;
        }

        Ok(result)
    }

    pub fn set_task_status(&self, command: UpdateTaskStatusCommand) -> Result<Task, AppError> {
        ensure_active_status(command.status)?;

        let mut task = self.load_task_by_ref(&command.task_ref)?;
        task.status = command.status;
        task.touch(OffsetDateTime::now_utc());
        self.repository.save_task(&task)?;
        Ok(task)
    }

    pub fn add_task_log(&self, command: AddTaskLogCommand) -> Result<Task, AppError> {
        let mut task = self.load_task_by_ref(&command.task_ref)?;
        let now = OffsetDateTime::now_utc();
        task.logs.push(TaskLog {
            at: now,
            message: command.message,
        });
        task.touch(now);
        self.repository.save_task(&task)?;
        Ok(task)
    }

    pub fn archive_task(&self, command: ArchiveTaskCommand) -> Result<OperationOutcome, AppError> {
        let all_tasks = self.repository.list_all_tasks()?;
        let root_task = self.resolve_task_from(&all_tasks, &command.task_ref)?;
        let subtree_ids = collect_subtree_ids(&all_tasks, &root_task.id);
        let now = OffsetDateTime::now_utc();

        let updated_tasks = all_tasks
            .iter()
            .filter(|task| subtree_ids.contains(&task.id))
            .cloned()
            .map(|mut task| {
                task.status = TaskStatus::Archived;
                task.touch(now);
                task
            })
            .collect::<Vec<_>>();

        self.maintenance_service.execute_operation(
            PendingOperationKind::TaskArchive,
            updated_tasks
                .iter()
                .cloned()
                .map(PendingOperationEntry::TaskUpsert)
                .collect(),
        )?;

        let root_task = self.repository.load_task(&root_task.id)?;
        Ok(OperationOutcome {
            root_task: Some(root_task),
            root_space: None,
            affected_count: updated_tasks.len(),
        })
    }

    pub fn restore_task(&self, command: RestoreTaskCommand) -> Result<OperationOutcome, AppError> {
        ensure_active_status(command.status)?;

        let all_tasks = self.repository.list_all_tasks()?;
        let root_task = self.resolve_task_from(&all_tasks, &command.task_ref)?;
        if !root_task.status.is_archived() {
            return Err(AppError::TaskMustBeArchived {
                task_id: root_task.id,
                action: "restore",
            });
        }

        if let Some(ancestor_id) = first_archived_ancestor_id(&all_tasks, &root_task) {
            return Err(AppError::TaskRestoreBlockedByArchivedAncestor {
                task_id: root_task.id,
                ancestor_id,
            });
        }

        let subtree_ids = collect_subtree_ids(&all_tasks, &root_task.id);
        let now = OffsetDateTime::now_utc();
        let updated_tasks = all_tasks
            .iter()
            .filter(|task| subtree_ids.contains(&task.id))
            .cloned()
            .map(|mut task| {
                task.status = command.status;
                task.touch(now);
                task
            })
            .collect::<Vec<_>>();

        self.maintenance_service.execute_operation(
            PendingOperationKind::TaskRestore,
            updated_tasks
                .iter()
                .cloned()
                .map(PendingOperationEntry::TaskUpsert)
                .collect(),
        )?;

        let root_task = self.repository.load_task(&root_task.id)?;
        Ok(OperationOutcome {
            root_task: Some(root_task),
            root_space: None,
            affected_count: updated_tasks.len(),
        })
    }

    pub fn purge_task(&self, command: PurgeTaskCommand) -> Result<OperationOutcome, AppError> {
        let all_tasks = self.repository.list_all_tasks()?;
        let root_task = self.resolve_task_from(&all_tasks, &command.task_ref)?;
        if !root_task.status.is_archived() {
            return Err(AppError::TaskMustBeArchived {
                task_id: root_task.id.clone(),
                action: "purge",
            });
        }

        let subtree_ids = collect_subtree_ids(&all_tasks, &root_task.id);
        if subtree_ids.len() > 1 && !command.recursive {
            return Err(AppError::TaskPurgeRequiresRecursive {
                task_id: root_task.id.clone(),
                child_count: subtree_ids.len() - 1,
            });
        }

        if let Some(offender) = all_tasks
            .iter()
            .find(|task| subtree_ids.contains(&task.id) && !task.status.is_archived())
        {
            return Err(AppError::TaskPurgeRequiresArchivedSubtree {
                task_id: root_task.id.clone(),
                offender_id: offender.id.clone(),
            });
        }

        let mut tasks_to_delete = all_tasks
            .iter()
            .filter(|task| subtree_ids.contains(&task.id))
            .cloned()
            .collect::<Vec<_>>();
        tasks_to_delete.sort_by_key(|task| std::cmp::Reverse(task.created_at));

        self.maintenance_service.execute_operation(
            PendingOperationKind::TaskPurge,
            tasks_to_delete
                .iter()
                .map(|task| PendingOperationEntry::TaskDelete {
                    task_id: task.id.clone(),
                })
                .collect(),
        )?;

        Ok(OperationOutcome {
            root_task: Some(root_task),
            root_space: None,
            affected_count: tasks_to_delete.len(),
        })
    }

    pub fn move_task(&self, command: MoveTaskCommand) -> Result<Task, AppError> {
        let all_tasks = self.repository.list_all_tasks()?;
        let task = self.resolve_task_from(&all_tasks, &command.task_ref)?;
        let space = self.repository.load_space(&task.space_id)?;
        ensure_active_space(&space)?;

        let mut siblings = all_tasks
            .iter()
            .filter(|candidate| candidate.space_id == task.space_id)
            .filter(|candidate| candidate.parent_id == task.parent_id)
            .cloned()
            .collect::<Vec<_>>();
        sort_tasks_in_place(&mut siblings, SortMode::Manual);

        let Some(index) = siblings
            .iter()
            .position(|candidate| candidate.id == task.id)
        else {
            return Ok(task);
        };

        let swap_index = match command.direction {
            MoveTaskDirection::Up => index.checked_sub(1),
            MoveTaskDirection::Down => (index + 1 < siblings.len()).then_some(index + 1),
        }
        .ok_or_else(|| AppError::TaskReorderBoundary {
            task_id: task.id.clone(),
            direction: move_direction_label(command.direction),
        })?;

        siblings.swap(index, swap_index);
        let now = OffsetDateTime::now_utc();

        for (sort_order, sibling) in siblings.iter_mut().enumerate() {
            let next_sort_order = sort_order as i64;
            if sibling.sort_order != next_sort_order {
                sibling.sort_order = next_sort_order;
                sibling.touch(now);
                self.repository.save_task(sibling)?;
            }
        }

        siblings
            .into_iter()
            .find(|candidate| candidate.id == task.id)
            .ok_or_else(|| {
                AppError::Reference(crate::domain::ReferenceError::TaskNotFound(
                    command.task_ref,
                ))
            })
    }

    pub fn load_task(&self, task_id: &TaskId) -> Result<Task, AppError> {
        self.repository.load_task(task_id).map_err(AppError::from)
    }

    pub fn save_task(&self, task: &Task) -> Result<(), AppError> {
        self.repository.save_task(task).map_err(AppError::from)
    }

    fn load_task_by_ref(&self, reference: &str) -> Result<Task, AppError> {
        let all_tasks = self.repository.list_all_tasks()?;
        self.resolve_task_from(&all_tasks, reference)
    }

    fn resolve_task_from(&self, tasks: &[Task], reference: &str) -> Result<Task, AppError> {
        let task_id = resolve_task_ref(reference, tasks.iter().map(|task| &task.id))?;
        tasks
            .iter()
            .find(|task| task.id == task_id)
            .cloned()
            .ok_or_else(|| {
                AppError::Reference(crate::domain::ReferenceError::TaskNotFound(
                    reference.to_owned(),
                ))
            })
    }

    fn resolve_space(&self, reference: &str, require_active: bool) -> Result<Space, AppError> {
        let spaces = self.repository.list_spaces()?;
        let space_id = crate::domain::resolve_space_ref(reference, spaces.iter())?;
        let space = spaces
            .into_iter()
            .find(|space| space.id == space_id)
            .expect("resolved space id must exist");
        if require_active {
            ensure_active_space(&space)?;
        }
        Ok(space)
    }

    fn resolve_effective_space(
        &self,
        reference: Option<&str>,
        require_active: bool,
    ) -> Result<Space, AppError> {
        if let Some(reference) = reference {
            return self.resolve_space(reference, require_active);
        }

        let state = self.repository.load_state()?;
        let current_space_id = state
            .current_space_id
            .ok_or(AppError::MissingCurrentSpace)?;
        let space = self.repository.load_space(&current_space_id)?;
        if require_active {
            ensure_active_space(&space)?;
        }
        Ok(space)
    }
}

pub(crate) fn ensure_active_status(status: TaskStatus) -> Result<(), AppError> {
    if status.is_archived() {
        Err(AppError::ArchivedStatusNotAllowed)
    } else {
        Ok(())
    }
}

pub(crate) fn ensure_active_space(space: &Space) -> Result<(), AppError> {
    if !space.state.is_active() {
        Err(AppError::ArchivedSpace(space.id.as_str().to_owned()))
    } else {
        Ok(())
    }
}

pub(crate) fn next_sort_order(
    tasks: &[Task],
    space_id: &crate::domain::SpaceId,
    parent_id: Option<&TaskId>,
    excluded_ids: &HashSet<TaskId>,
) -> i64 {
    tasks
        .iter()
        .filter(|task| &task.space_id == space_id)
        .filter(|task| task.parent_id.as_ref() == parent_id)
        .filter(|task| !excluded_ids.contains(&task.id))
        .map(|task| task.sort_order)
        .max()
        .unwrap_or(-1)
        + 1
}

pub(crate) fn collect_subtree_ids(tasks: &[Task], root_id: &TaskId) -> HashSet<TaskId> {
    let mut stack = vec![root_id.clone()];
    let mut seen = HashSet::new();

    while let Some(current_id) = stack.pop() {
        if !seen.insert(current_id.clone()) {
            continue;
        }

        for child in tasks
            .iter()
            .filter(|task| task.parent_id.as_ref() == Some(&current_id))
        {
            stack.push(child.id.clone());
        }
    }

    seen
}

fn first_archived_ancestor_id(tasks: &[Task], task: &Task) -> Option<TaskId> {
    let mut parent_id = task.parent_id.clone();
    while let Some(current_parent_id) = parent_id {
        let parent = tasks
            .iter()
            .find(|candidate| candidate.id == current_parent_id)?;
        if parent.status.is_archived() {
            return Some(parent.id.clone());
        }
        parent_id = parent.parent_id.clone();
    }

    None
}

fn move_direction_label(direction: MoveTaskDirection) -> &'static str {
    match direction {
        MoveTaskDirection::Up => "up",
        MoveTaskDirection::Down => "down",
    }
}
