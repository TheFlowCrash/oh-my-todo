use crate::application::queries::{TaskListEntry, TaskListResult};
use crate::domain::{SortMode, Space, SpaceCounts, Task, TaskStatus, ViewMode};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

pub fn derive_space_counts(tasks: &[Task]) -> SpaceCounts {
    SpaceCounts {
        todo_tasks: tasks.iter().filter(|task| !task.archived).count(),
        archived_tasks: tasks.iter().filter(|task| task.archived).count(),
    }
}

pub fn build_task_list(
    space: Space,
    tasks: Vec<Task>,
    view: ViewMode,
    sort: SortMode,
) -> TaskListResult {
    let visible_ids = tasks
        .iter()
        .filter(|task| task.is_visible_in_view(view))
        .map(|task| task.id.clone())
        .collect::<HashSet<_>>();

    let mut children: HashMap<Option<_>, Vec<Task>> = HashMap::new();
    for task in tasks
        .into_iter()
        .filter(|task| task.is_visible_in_view(view))
    {
        let parent_key = match task.parent_id.as_ref() {
            Some(parent_id) if visible_ids.contains(parent_id) => Some(parent_id.clone()),
            _ => None,
        };

        children.entry(parent_key).or_default().push(task);
    }

    for siblings in children.values_mut() {
        sort_tasks_in_place(siblings, sort);
    }

    let mut entries = Vec::new();
    flatten_children(None, 0, &children, &mut entries);

    TaskListResult {
        space,
        view,
        sort,
        entries,
    }
}

pub fn sort_tasks_in_place(tasks: &mut [Task], sort: SortMode) {
    tasks.sort_by(|left, right| compare_tasks(left, right, sort));
}

fn flatten_children(
    parent_id: Option<crate::domain::TaskId>,
    depth: usize,
    children: &HashMap<Option<crate::domain::TaskId>, Vec<Task>>,
    entries: &mut Vec<TaskListEntry>,
) {
    if let Some(siblings) = children.get(&parent_id) {
        for task in siblings {
            let child_count = children.get(&Some(task.id.clone())).map_or(0, Vec::len);
            entries.push(TaskListEntry {
                task: task.clone(),
                depth,
                child_count,
            });
            flatten_children(Some(task.id.clone()), depth + 1, children, entries);
        }
    }
}

fn compare_tasks(left: &Task, right: &Task, sort: SortMode) -> Ordering {
    match sort {
        SortMode::Created => left
            .created_at
            .cmp(&right.created_at)
            .then_with(|| left.sort_order.cmp(&right.sort_order)),
        SortMode::Updated => right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.sort_order.cmp(&right.sort_order)),
        SortMode::Status => left
            .archived
            .cmp(&right.archived)
            .then_with(|| status_rank(left.status).cmp(&status_rank(right.status)))
            .then_with(|| left.sort_order.cmp(&right.sort_order)),
        SortMode::Manual => left.sort_order.cmp(&right.sort_order),
    }
    .then_with(|| left.created_at.cmp(&right.created_at))
    .then_with(|| left.id.as_str().cmp(right.id.as_str()))
}

fn status_rank(status: TaskStatus) -> u8 {
    match status {
        TaskStatus::Todo => 0,
        TaskStatus::InProgress => 1,
        TaskStatus::Done => 2,
        TaskStatus::Close => 3,
    }
}
