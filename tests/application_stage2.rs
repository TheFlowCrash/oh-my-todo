use oh_my_todo::application::bootstrap::{BootstrapOptions, bootstrap};
use oh_my_todo::application::commands::{
    CreateSpaceCommand, CreateTaskCommand, EditTaskCommand, RenameSpaceCommand,
    SetCurrentSpaceCommand,
};
use oh_my_todo::application::error::AppError;
use tempfile::tempdir;

#[test]
fn rename_space_keeps_slug_and_use_space_updates_state() {
    let temp_dir = tempdir().unwrap();
    let context = bootstrap(BootstrapOptions {
        data_root: Some(temp_dir.path().join("app_data")),
    })
    .unwrap();

    let space = context
        .space_service
        .create_space(CreateSpaceCommand {
            name: "Personal Workspace".to_owned(),
        })
        .unwrap();
    let original_slug = space.slug.clone();

    let renamed = context
        .space_service
        .rename_space(RenameSpaceCommand {
            space_ref: space.slug.clone(),
            new_name: "Personal Focus".to_owned(),
        })
        .unwrap();
    let current = context
        .space_service
        .use_space(SetCurrentSpaceCommand {
            space_ref: renamed.id.short_id(),
        })
        .unwrap();
    let state = context.space_service.load_app_state().unwrap();

    assert_eq!(renamed.slug, original_slug);
    assert_eq!(current.id, renamed.id);
    assert_eq!(state.current_space_id, Some(renamed.id));
}

#[test]
fn edit_task_rejects_cycles_and_cross_space_parent_mismatch() {
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
    let work = context
        .space_service
        .create_space(CreateSpaceCommand {
            name: "Work".to_owned(),
        })
        .unwrap();

    let parent = context
        .task_service
        .create_task(CreateTaskCommand {
            title: "Parent".to_owned(),
            space_ref: Some(personal.slug.clone()),
            description: None,
            parent_ref: None,
            status: oh_my_todo::domain::TaskStatus::Todo,
        })
        .unwrap();
    let child = context
        .task_service
        .create_task(CreateTaskCommand {
            title: "Child".to_owned(),
            space_ref: None,
            description: None,
            parent_ref: Some(parent.id.short_id()),
            status: oh_my_todo::domain::TaskStatus::Todo,
        })
        .unwrap();

    let cycle_error = context
        .task_service
        .edit_task(EditTaskCommand {
            task_ref: parent.id.as_str().to_owned(),
            title: None,
            description: None,
            clear_description: false,
            status: None,
            parent_ref: Some(child.id.as_str().to_owned()),
            clear_parent: false,
            space_ref: None,
        })
        .unwrap_err();
    assert!(matches!(cycle_error, AppError::TaskParentCycle { .. }));

    let move_error = context
        .task_service
        .edit_task(EditTaskCommand {
            task_ref: child.id.as_str().to_owned(),
            title: None,
            description: None,
            clear_description: false,
            status: None,
            parent_ref: None,
            clear_parent: false,
            space_ref: Some(work.slug.clone()),
        })
        .unwrap_err();
    assert!(matches!(
        move_error,
        AppError::CrossSpaceParentMismatch { .. } | AppError::ParentSpaceMismatch { .. }
    ));
}
