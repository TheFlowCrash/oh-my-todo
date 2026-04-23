use oh_my_todo::domain::{AppConfig, AppState, Space, Task, TaskLog, TaskStatus};
use oh_my_todo::storage::serializer::{from_ron_str, to_ron_string};
use time::OffsetDateTime;

#[test]
fn ron_round_trip_preserves_space_task_config_and_state() {
    let mut space = Space::new("Personal Workspace", 0);
    let mut task = Task::new("Run 5km", space.id.clone(), 0);
    task.description = Some("Finish prep for Sunday run".to_owned());
    task.status = TaskStatus::InProgress;
    task.logs.push(TaskLog {
        at: OffsetDateTime::now_utc(),
        message: "Warm-up complete".to_owned(),
    });

    let mut state = AppState::default();
    state.current_space_id = Some(space.id.clone());
    let config = AppConfig::default();

    let space_ron = to_ron_string(&space).unwrap();
    let task_ron = to_ron_string(&task).unwrap();
    let config_ron = to_ron_string(&config).unwrap();
    let state_ron = to_ron_string(&state).unwrap();

    let decoded_space: Space = from_ron_str(&space_ron).unwrap();
    let decoded_task: Task = from_ron_str(&task_ron).unwrap();
    let decoded_config: AppConfig = from_ron_str(&config_ron).unwrap();
    let decoded_state: AppState = from_ron_str(&state_ron).unwrap();

    assert_eq!(decoded_space, space);
    assert_eq!(decoded_task, task);
    assert_eq!(decoded_config, config);
    assert_eq!(decoded_state, state);

    space.slug = "custom_slug".to_owned();
    assert_ne!(decoded_space, space);
}
