use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn cli_stage3_task_archive_restore_and_purge_flow() {
    let temp_dir = tempdir().unwrap();
    let data_dir = temp_dir.path().join("app_data");

    run(&data_dir, ["space", "add", "personal"]);
    run(&data_dir, ["space", "use", "personal"]);

    let add_parent = run(&data_dir, ["task", "add", "Run 5km"]);
    let parent_id = extract_id(&add_parent, "tsk_");
    run(
        &data_dir,
        ["task", "add", "--parent", parent_id.as_str(), "Warm up"],
    );

    let archived = run(&data_dir, ["task", "archive", parent_id.as_str()]);
    assert!(archived.contains("Archived task"));

    let archive_list = run(&data_dir, ["task", "list", "--view", "archive"]);
    assert!(archive_list.contains("[a] Run 5km"));
    assert!(archive_list.contains("Warm up"));

    let restored = run(
        &data_dir,
        [
            "task",
            "restore",
            parent_id.as_str(),
            "--status",
            "in_progress",
        ],
    );
    assert!(restored.contains("Restored task"));
    let todo_list = run(&data_dir, ["task", "list"]);
    assert!(todo_list.contains("[~] Run 5km"));
    assert!(todo_list.contains("[~] Warm up"));

    run(&data_dir, ["task", "archive", parent_id.as_str()]);

    Command::cargo_bin("oh-my-todo")
        .unwrap()
        .env("OH_MY_TODO_DATA_DIR", &data_dir)
        .args(["task", "purge", parent_id.as_str(), "--force"])
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("--recursive"));

    let purged = run(
        &data_dir,
        [
            "task",
            "purge",
            parent_id.as_str(),
            "--force",
            "--recursive",
        ],
    );
    assert!(purged.contains("Purged task"));

    let all_list = run(&data_dir, ["task", "list", "--view", "all"]);
    assert!(!all_list.contains("Run 5km"));
    assert!(!all_list.contains("Warm up"));
}

#[test]
fn cli_stage3_space_lifecycle_and_doctor_flow() {
    let temp_dir = tempdir().unwrap();
    let data_dir = temp_dir.path().join("app_data");

    run(&data_dir, ["space", "add", "personal"]);
    run(&data_dir, ["space", "add", "work"]);
    run(&data_dir, ["space", "use", "personal"]);

    let archived = run(&data_dir, ["space", "archive", "personal"]);
    assert!(archived.contains("Archived space"));

    let listed = run(&data_dir, ["space", "list"]);
    assert!(!listed.contains("personal"));
    assert!(listed.contains("* work"));

    let restored = run(&data_dir, ["space", "restore", "personal"]);
    assert!(restored.contains("Restored space"));

    run(&data_dir, ["space", "archive", "personal"]);
    let purged = run(&data_dir, ["space", "purge", "personal", "--force"]);
    assert!(purged.contains("Purged space"));

    let doctor = run(&data_dir, ["doctor"]);
    assert!(doctor.contains("No problems found."));
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
