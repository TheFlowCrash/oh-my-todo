use oh_my_todo::application::bootstrap::{BootstrapOptions, bootstrap};
use oh_my_todo::domain::{Space, Task, TaskStatus};
use std::fs;
use tempfile::tempdir;

#[test]
fn bootstrap_initializes_empty_repository_layout() {
    let temp_dir = tempdir().unwrap();
    let data_root = temp_dir.path().join("app_data");

    let context = bootstrap(BootstrapOptions {
        data_root: Some(data_root.clone()),
    })
    .unwrap();

    assert_eq!(context.data_root(), data_root.as_path());
    assert!(data_root.join("config").exists());
    assert!(data_root.join("config").join("config.ron").exists());
    assert!(data_root.join("config").join("state.ron").exists());
    assert!(data_root.join("spaces").exists());
    assert!(context.startup.spaces.is_empty());
    assert!(context.space_service.list_spaces().unwrap().is_empty());
}

#[test]
fn repository_round_trip_through_services_creates_expected_files() {
    let temp_dir = tempdir().unwrap();
    let data_root = temp_dir.path().join("app_data");
    let context = bootstrap(BootstrapOptions {
        data_root: Some(data_root.clone()),
    })
    .unwrap();

    let space = Space::new("Personal", 0);
    context.space_service.save_space(&space).unwrap();

    let mut task = Task::new("Run 5km", space.id.clone(), 0);
    task.status = TaskStatus::Done;
    context.task_service.save_task(&task).unwrap();

    let reloaded_space = context.space_service.load_space(&space.id).unwrap();
    let reloaded_task = context.task_service.load_task(&task.id).unwrap();
    let disk_task = fs::read_to_string(
        data_root
            .join("spaces")
            .join(space.id.as_str())
            .join("todo")
            .join(format!("{}.ron", task.id.as_str())),
    )
    .unwrap();

    assert_eq!(reloaded_space, space);
    assert_eq!(reloaded_task, task);
    assert!(disk_task.contains("Run 5km"));
}
