use oh_my_todo::application::bootstrap::{BootstrapOptions, bootstrap};
use oh_my_todo::application::commands::{
    ArchiveSpaceCommand, CreateSpaceCommand, CreateTaskCommand, MoveTaskCommand, MoveTaskDirection,
    SetCurrentSpaceCommand,
};
use oh_my_todo::application::error::AppError;
use oh_my_todo::application::queries::ListTasksQuery;
use oh_my_todo::domain::{SortMode, SpaceListMode, TaskStatus, ViewMode};
use tempfile::tempdir;

#[test]
fn move_task_reorders_manual_siblings_and_reports_boundaries() {
    let temp_dir = tempdir().unwrap();
    let context = bootstrap(BootstrapOptions {
        data_root: Some(temp_dir.path().join("app_data")),
    })
    .unwrap();

    let personal = context
        .space_service
        .create_space(CreateSpaceCommand {
            name: "Personal".to_owned(),
        })
        .unwrap();
    context
        .space_service
        .use_space(SetCurrentSpaceCommand {
            space_ref: personal.id.as_str().to_owned(),
        })
        .unwrap();

    let first = context
        .task_service
        .create_task(CreateTaskCommand {
            title: "First".to_owned(),
            space_ref: None,
            description: None,
            parent_ref: None,
            status: TaskStatus::Todo,
        })
        .unwrap();
    let _second = context
        .task_service
        .create_task(CreateTaskCommand {
            title: "Second".to_owned(),
            space_ref: None,
            description: None,
            parent_ref: None,
            status: TaskStatus::Todo,
        })
        .unwrap();
    let third = context
        .task_service
        .create_task(CreateTaskCommand {
            title: "Third".to_owned(),
            space_ref: None,
            description: None,
            parent_ref: None,
            status: TaskStatus::Todo,
        })
        .unwrap();

    context
        .task_service
        .move_task(MoveTaskCommand {
            task_ref: third.id.as_str().to_owned(),
            direction: MoveTaskDirection::Up,
        })
        .unwrap();

    let listed = context
        .task_service
        .list_tasks(ListTasksQuery {
            space_ref: Some(personal.id.as_str().to_owned()),
            view: Some(ViewMode::All),
            sort: Some(SortMode::Manual),
            allow_archived_space: false,
        })
        .unwrap();

    let titles = listed
        .entries
        .into_iter()
        .map(|entry| entry.task.title)
        .collect::<Vec<_>>();
    assert_eq!(titles, vec!["First", "Third", "Second"]);

    let error = context
        .task_service
        .move_task(MoveTaskCommand {
            task_ref: first.id.as_str().to_owned(),
            direction: MoveTaskDirection::Up,
        })
        .unwrap_err();
    assert!(matches!(error, AppError::TaskReorderBoundary { .. }));
}

#[test]
fn archived_space_queries_and_tui_state_preferences_round_trip() {
    let temp_dir = tempdir().unwrap();
    let context = bootstrap(BootstrapOptions {
        data_root: Some(temp_dir.path().join("app_data")),
    })
    .unwrap();

    let active = context
        .space_service
        .create_space(CreateSpaceCommand {
            name: "Active".to_owned(),
        })
        .unwrap();
    let archived = context
        .space_service
        .create_space(CreateSpaceCommand {
            name: "Someday".to_owned(),
        })
        .unwrap();

    context
        .space_service
        .use_space(SetCurrentSpaceCommand {
            space_ref: archived.id.as_str().to_owned(),
        })
        .unwrap();
    context
        .task_service
        .create_task(CreateTaskCommand {
            title: "Review later".to_owned(),
            space_ref: None,
            description: Some("keep for archive browsing".to_owned()),
            parent_ref: None,
            status: TaskStatus::Todo,
        })
        .unwrap();
    context
        .space_service
        .use_space(SetCurrentSpaceCommand {
            space_ref: active.id.as_str().to_owned(),
        })
        .unwrap();
    context
        .space_service
        .archive_space(ArchiveSpaceCommand {
            space_ref: archived.id.as_str().to_owned(),
        })
        .unwrap();

    let report = context
        .task_service
        .list_tasks(ListTasksQuery {
            space_ref: Some(archived.id.as_str().to_owned()),
            view: Some(ViewMode::All),
            sort: Some(SortMode::Manual),
            allow_archived_space: true,
        })
        .unwrap();
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].task.title, "Review later");

    let archived_error = context
        .task_service
        .list_tasks(ListTasksQuery {
            space_ref: Some(archived.id.as_str().to_owned()),
            view: Some(ViewMode::All),
            sort: Some(SortMode::Manual),
            allow_archived_space: false,
        })
        .unwrap_err();
    assert!(matches!(archived_error, AppError::ArchivedSpace(_)));

    context
        .app_state_service
        .update(|state| {
            state.tui_memory.selected_space_id = Some(archived.id.clone());
            state.tui_memory.space_list_mode = SpaceListMode::All;
            state.tui_memory.task_filter = "review".to_owned();
        })
        .unwrap();

    let reloaded_state = context.app_state_service.load().unwrap();
    assert_eq!(reloaded_state.current_space_id, Some(active.id));
    assert_eq!(
        reloaded_state.tui_memory.selected_space_id,
        Some(archived.id)
    );
    assert_eq!(
        reloaded_state.tui_memory.space_list_mode,
        SpaceListMode::All
    );
    assert_eq!(reloaded_state.tui_memory.task_filter, "review");
}
