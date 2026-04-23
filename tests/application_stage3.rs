use oh_my_todo::application::bootstrap::{BootstrapOptions, bootstrap};
use oh_my_todo::application::commands::{
    ArchiveSpaceCommand, ArchiveTaskCommand, CreateSpaceCommand, CreateTaskCommand,
    RestoreTaskCommand, SetCurrentSpaceCommand,
};
use oh_my_todo::application::error::AppError;
use oh_my_todo::domain::{
    PendingOperation, PendingOperationEntry, PendingOperationKind, Task, TaskId, TaskStatus,
};
use oh_my_todo::storage::serializer::write_ron_file;
use oh_my_todo::storage::{AppRepository, DataPaths, FilesystemRepository};
use tempfile::tempdir;
use time::OffsetDateTime;

#[test]
fn restore_applies_status_to_whole_subtree_and_blocks_archived_child_restore() {
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
            space_ref: personal.slug.clone(),
        })
        .unwrap();

    let parent = context
        .task_service
        .create_task(CreateTaskCommand {
            title: "Parent".to_owned(),
            space_ref: None,
            description: None,
            parent_ref: None,
            status: TaskStatus::Todo,
        })
        .unwrap();
    let child = context
        .task_service
        .create_task(CreateTaskCommand {
            title: "Child".to_owned(),
            space_ref: None,
            description: None,
            parent_ref: Some(parent.id.as_str().to_owned()),
            status: TaskStatus::Todo,
        })
        .unwrap();

    context
        .task_service
        .archive_task(ArchiveTaskCommand {
            task_ref: parent.id.as_str().to_owned(),
        })
        .unwrap();

    let error = context
        .task_service
        .restore_task(RestoreTaskCommand {
            task_ref: child.id.as_str().to_owned(),
            status: TaskStatus::Todo,
        })
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::TaskRestoreBlockedByArchivedAncestor { .. }
    ));

    context
        .task_service
        .restore_task(RestoreTaskCommand {
            task_ref: parent.id.as_str().to_owned(),
            status: TaskStatus::InProgress,
        })
        .unwrap();

    let reloaded_parent = context.task_service.load_task(&parent.id).unwrap();
    let reloaded_child = context.task_service.load_task(&child.id).unwrap();
    assert_eq!(reloaded_parent.status, TaskStatus::InProgress);
    assert_eq!(reloaded_child.status, TaskStatus::InProgress);
}

#[test]
fn space_archive_switches_current_space_and_bootstrap_recovers_pending_operation() {
    let temp_dir = tempdir().unwrap();
    let data_root = temp_dir.path().join("app_data");
    let context = bootstrap(BootstrapOptions {
        data_root: Some(data_root.clone()),
    })
    .unwrap();

    let personal = context
        .space_service
        .create_space(CreateSpaceCommand {
            name: "Personal".to_owned(),
        })
        .unwrap();
    let work = context
        .space_service
        .create_space(CreateSpaceCommand {
            name: "Work".to_owned(),
        })
        .unwrap();
    context
        .space_service
        .use_space(SetCurrentSpaceCommand {
            space_ref: personal.slug.clone(),
        })
        .unwrap();

    let archive_outcome = context
        .space_service
        .archive_space(ArchiveSpaceCommand {
            space_ref: personal.slug.clone(),
        })
        .unwrap();
    let state = context.space_service.load_app_state().unwrap();
    let archived_space = archive_outcome.root_space.unwrap();
    assert_eq!(
        archived_space.state,
        oh_my_todo::domain::SpaceState::Archived
    );
    assert_eq!(state.current_space_id, Some(work.id.clone()));

    let repository = FilesystemRepository::new(DataPaths::from_root(data_root.clone()));
    repository.initialize().unwrap();

    let mut task = Task::new("Run 5km", work.id.clone(), 0);
    repository.save_task(&task).unwrap();
    task.status = TaskStatus::Archived;
    task.touch(OffsetDateTime::now_utc());

    let mut app_state = repository.load_state().unwrap();
    app_state.pending_operation = Some(PendingOperation {
        operation_id: "op_test".to_owned(),
        kind: PendingOperationKind::TaskArchive,
        created_at: OffsetDateTime::now_utc(),
        entries: vec![PendingOperationEntry::TaskUpsert(task.clone())],
    });
    repository.save_state(&app_state).unwrap();

    let recovered = bootstrap(BootstrapOptions {
        data_root: Some(data_root.clone()),
    })
    .unwrap();
    let recovered_task = recovered.task_service.load_task(&task.id).unwrap();
    let recovered_state = recovered.space_service.load_app_state().unwrap();

    assert_eq!(recovered_task.status, TaskStatus::Archived);
    assert!(
        data_root
            .join("spaces")
            .join(work.id.as_str())
            .join("archive")
            .join(format!("{}.ron", task.id.as_str()))
            .exists()
    );
    assert!(recovered_state.pending_operation.is_none());
}

#[test]
fn doctor_reports_missing_parent_and_bucket_status_mismatch() {
    let temp_dir = tempdir().unwrap();
    let data_root = temp_dir.path().join("app_data");
    let context = bootstrap(BootstrapOptions {
        data_root: Some(data_root.clone()),
    })
    .unwrap();

    let space = context
        .space_service
        .create_space(CreateSpaceCommand {
            name: "Personal".to_owned(),
        })
        .unwrap();

    let repository = FilesystemRepository::new(DataPaths::from_root(data_root.clone()));
    repository.initialize().unwrap();

    let mut orphan = Task::new("Orphan", space.id.clone(), 0);
    orphan.parent_id = Some(TaskId::new());
    repository.save_task(&orphan).unwrap();

    let mut mismatched = Task::new("Mismatched", space.id.clone(), 1);
    mismatched.status = TaskStatus::Archived;
    write_ron_file(
        &DataPaths::from_root(data_root)
            .space_todo_dir(&space.id)
            .join(format!("{}.ron", mismatched.id.as_str())),
        &mismatched,
    )
    .unwrap();

    let report = context.maintenance_service.doctor().unwrap();
    assert!(report.issues.iter().any(|issue| matches!(
        issue,
        oh_my_todo::application::queries::DoctorIssue::MissingParent { .. }
    )));
    assert!(report.issues.iter().any(|issue| matches!(
        issue,
        oh_my_todo::application::queries::DoctorIssue::BucketStatusMismatch { .. }
    )));
}
