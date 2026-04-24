use crate::application::queries::{
    DoctorIssue, DoctorReport, SpaceDetails, SpaceSummary, TaskDetails, TaskListResult,
};
use crate::domain::{Space, SpaceState, Task, TaskStatus};
use crate::storage::TaskBucket;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub fn format_created_space(space: &Space) -> String {
    format!(
        "Created space [{}] {} ({})",
        space.id.short_id(),
        space.name,
        space.id
    )
}

pub fn format_used_space(space: &Space) -> String {
    format!(
        "Set current space to [{}] {} ({})",
        space.id.short_id(),
        space.name,
        space.id
    )
}

pub fn format_renamed_space(space: &Space) -> String {
    format!(
        "Renamed space [{}] {} ({})",
        space.id.short_id(),
        space.name,
        space.id
    )
}

pub fn format_created_task(task: &Task, space: &Space) -> String {
    format!(
        "Created task [{}] {} ({}) in space {}",
        task.id.short_id(),
        task.title,
        task.id,
        space.slug
    )
}

pub fn format_updated_task(task: &Task) -> String {
    format!(
        "Updated task [{}] {} ({})",
        task.id.short_id(),
        task.title,
        task.id
    )
}

pub fn format_task_status(task: &Task) -> String {
    let archived_suffix = if task.archived { " and archived" } else { "" };
    format!(
        "Set task [{}] {} ({}) to {}{}",
        task.id.short_id(),
        task.title,
        task.id,
        format_task_status_name(task.status),
        archived_suffix,
    )
}

pub fn format_logged_task(task: &Task) -> String {
    format!(
        "Added log to task [{}] {} ({})",
        task.id.short_id(),
        task.title,
        task.id
    )
}

pub fn format_archived_task(task: &Task, affected_count: usize) -> String {
    format!(
        "Archived task [{}] {} ({}) across {} task(s)",
        task.id.short_id(),
        task.title,
        task.id,
        affected_count
    )
}

pub fn format_restored_task(task: &Task, affected_count: usize) -> String {
    format!(
        "Restored task [{}] {} ({}) with status {} across {} task(s)",
        task.id.short_id(),
        task.title,
        task.id,
        format_task_status_name(task.status),
        affected_count
    )
}

pub fn format_purged_task(task: &Task, affected_count: usize) -> String {
    format!(
        "Purged task [{}] {} ({}) across {} task(s)",
        task.id.short_id(),
        task.title,
        task.id,
        affected_count
    )
}

pub fn format_archived_space(space: &Space) -> String {
    format!(
        "Archived space [{}] {} ({})",
        space.id.short_id(),
        space.name,
        space.id
    )
}

pub fn format_restored_space(space: &Space) -> String {
    format!(
        "Restored space [{}] {} ({})",
        space.id.short_id(),
        space.name,
        space.id
    )
}

pub fn format_purged_space(space: &Space) -> String {
    format!(
        "Purged space [{}] {} ({})",
        space.id.short_id(),
        space.name,
        space.id
    )
}

pub fn render_spaces(items: &[SpaceSummary]) -> String {
    if items.is_empty() {
        return "No spaces found.".to_owned();
    }

    items
        .iter()
        .map(|item| {
            format!(
                "{} {} [{}] slug: {} state: {} tasks: {} active / {} archived",
                if item.is_current { "*" } else { "-" },
                item.space.name,
                item.space.id.short_id(),
                item.space.slug,
                format_space_state(item.space.state),
                item.counts.todo_tasks,
                item.counts.archived_tasks,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_space(details: &SpaceDetails) -> String {
    [
        format!("Name: {}", details.space.name),
        format!("Space ID: {}", details.space.id),
        format!("Short ID: {}", details.space.id.short_id()),
        format!("Slug: {}", details.space.slug),
        format!("State: {}", format_space_state(details.space.state)),
        format!("Current: {}", yes_no(details.is_current)),
        format!(
            "Tasks: {} active / {} archived",
            details.counts.todo_tasks, details.counts.archived_tasks
        ),
        format!("Created: {}", format_timestamp(details.space.created_at)),
        format!("Updated: {}", format_timestamp(details.space.updated_at)),
    ]
    .join("\n")
}

pub fn render_task_list(result: &TaskListResult) -> String {
    let mut lines = vec![
        format!(
            "Space: {} [{}]",
            result.space.name,
            result.space.id.short_id()
        ),
        format!("View: {}", format_view_name(result.view)),
        format!("Sort: {}", format_sort_name(result.sort)),
        String::new(),
    ];

    if result.entries.is_empty() {
        lines.push("No tasks found.".to_owned());
        return lines.join("\n");
    }

    for entry in &result.entries {
        let indent = "  ".repeat(entry.depth);
        let branch = if entry.depth == 0 { "" } else { "- " };
        lines.push(format!(
            "{}{}{} {} [{}]",
            indent,
            branch,
            status_marker(entry.task.status),
            entry.task.title,
            entry.task.id.short_id(),
        ));
    }

    lines.join("\n")
}

pub fn render_task(details: &TaskDetails) -> String {
    let parent = details
        .parent
        .as_ref()
        .map(|task| format!("{} [{}]", task.title, task.id.short_id()))
        .unwrap_or_else(|| "-".to_owned());

    let children = if details.children.is_empty() {
        "-".to_owned()
    } else {
        details
            .children
            .iter()
            .map(|task| {
                format!(
                    "{} {} [{}]",
                    status_marker(task.status),
                    task.title,
                    task.id.short_id()
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    let description = details
        .task
        .description
        .clone()
        .unwrap_or_else(|| "-".to_owned());

    let logs = if details.logs.is_empty() {
        "-".to_owned()
    } else {
        details
            .logs
            .iter()
            .map(|log| format!("- {} {}", format_timestamp(log.at), log.message))
            .collect::<Vec<_>>()
            .join("\n")
    };

    [
        format!("Title: {}", details.task.title),
        format!("Task ID: {}", details.task.id),
        format!("Short ID: {}", details.task.id.short_id()),
        format!(
            "Space: {} [{}] ({})",
            details.space.name,
            details.space.id.short_id(),
            details.space.slug
        ),
        format!("Status: {}", format_task_status_name(details.task.status)),
        format!("Archived: {}", yes_no(details.task.archived)),
        format!("Parent: {}", parent),
        format!("Children: {}", children),
        format!("Created: {}", format_timestamp(details.task.created_at)),
        format!("Updated: {}", format_timestamp(details.task.updated_at)),
        format!("Description: {}", description),
        "Recent Logs:".to_owned(),
        logs,
    ]
    .join("\n")
}

pub fn render_doctor_report(report: &DoctorReport) -> String {
    if report.issues.is_empty() {
        return "No problems found.".to_owned();
    }

    let mut lines = vec![format!("Problems found: {}", report.issues.len())];
    for issue in &report.issues {
        lines.push(format!("- {}", format_doctor_issue(issue)));
    }

    lines.join("\n")
}

pub fn format_task_status_name(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
        TaskStatus::Close => "close",
    }
}

pub fn format_view_name(view: crate::domain::ViewMode) -> &'static str {
    match view {
        crate::domain::ViewMode::Todo => "todo",
        crate::domain::ViewMode::Archive => "archive",
        crate::domain::ViewMode::All => "all",
    }
}

pub fn format_sort_name(sort: crate::domain::SortMode) -> &'static str {
    match sort {
        crate::domain::SortMode::Created => "created",
        crate::domain::SortMode::Updated => "updated",
        crate::domain::SortMode::Status => "status",
        crate::domain::SortMode::Manual => "manual",
    }
}

fn status_marker(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "[ ]",
        TaskStatus::InProgress => "[~]",
        TaskStatus::Done => "[x]",
        TaskStatus::Close => "[c]",
    }
}

fn format_space_state(state: SpaceState) -> &'static str {
    match state {
        SpaceState::Active => "active",
        SpaceState::Archived => "archived",
    }
}

fn format_doctor_issue(issue: &DoctorIssue) -> String {
    match issue {
        DoctorIssue::PendingOperation { operation_id, kind } => {
            format!("pending operation {operation_id} ({})", kind.as_str())
        }
        DoctorIssue::MissingParent { task_id, parent_id } => {
            format!("task {task_id} has missing parent {parent_id}")
        }
        DoctorIssue::CrossSpaceParent {
            task_id,
            task_space_id,
            parent_id,
            parent_space_id,
        } => format!(
            "task {task_id} in space {task_space_id} points to parent {parent_id} in space {parent_space_id}"
        ),
        DoctorIssue::ParentCycle { task_id } => {
            format!("task {task_id} participates in a parent cycle")
        }
        DoctorIssue::BucketStatusMismatch {
            task_id,
            bucket,
            archived,
        } => format!(
            "task {task_id} is stored in {} but archived is {}",
            format_bucket(*bucket),
            yes_no(*archived)
        ),
    }
}

fn format_bucket(bucket: TaskBucket) -> &'static str {
    match bucket {
        TaskBucket::Todo => "todo/",
        TaskBucket::Archive => "archive/",
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn format_timestamp(value: OffsetDateTime) -> String {
    value.format(&Rfc3339).unwrap_or_else(|_| value.to_string())
}
