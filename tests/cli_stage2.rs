use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn cli_stage2_core_flow_works_end_to_end() {
    let temp_dir = tempdir().unwrap();
    let data_dir = temp_dir.path().join("app_data");

    let add_space = run(&data_dir, ["space", "add", "personal"]);
    assert!(add_space.contains("Created space"));

    let use_space = run(&data_dir, ["space", "use", "personal"]);
    assert!(use_space.contains("Set current space"));

    let add_parent = run(&data_dir, ["task", "add", "Run 5km"]);
    let parent_id = extract_id(&add_parent, "tsk_");
    assert!(add_parent.contains("Created task"));

    let add_child = run(
        &data_dir,
        ["task", "add", "--parent", parent_id.as_str(), "Warm up"],
    );
    let child_id = extract_id(&add_child, "tsk_");
    assert!(add_child.contains("Created task"));

    let child_done = run(&data_dir, ["task", "done", child_id.as_str()]);
    assert!(child_done.contains("to done and archived"));

    let done = run(&data_dir, ["task", "done", parent_id.as_str()]);
    assert!(done.contains("to done and archived"));

    let archive_list = run(&data_dir, ["task", "list", "--view", "archive"]);
    assert!(archive_list.contains("[x] Run 5km"));
    assert!(archive_list.contains("[x] Warm up"));

    let log = run(
        &data_dir,
        [
            "task",
            "log",
            "add",
            parent_id.as_str(),
            "Today finished 5km",
        ],
    );
    assert!(log.contains("Added log to task"));

    let show = run(&data_dir, ["task", "show", parent_id.as_str()]);
    assert!(show.contains("Recent Logs:"));
    assert!(show.contains("Today finished 5km"));
}

#[test]
fn cli_requires_current_space_for_task_add_without_space_flag() {
    let temp_dir = tempdir().unwrap();
    let data_dir = temp_dir.path().join("app_data");

    Command::cargo_bin("oh-my-todo")
        .unwrap()
        .env("OH_MY_TODO_DATA_DIR", &data_dir)
        .args(["task", "add", "Run 5km"])
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("no current space selected"))
        .stderr(predicate::str::contains("todo space add <NAME>"));
}

fn run<const N: usize>(data_dir: &std::path::Path, args: [&str; N]) -> String {
    let output = Command::cargo_bin("oh-my-todo")
        .unwrap()
        .env("OH_MY_TODO_DATA_DIR", data_dir)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    String::from_utf8(output).unwrap()
}

fn extract_id(output: &str, prefix: &str) -> String {
    let start = output
        .rfind(prefix)
        .expect("output should contain a trackable id");
    let suffix = &output[start..];
    let end = suffix
        .find(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .expect("id should end before punctuation");
    suffix[..end].to_owned()
}
