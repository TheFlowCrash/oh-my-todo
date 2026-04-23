use oh_my_todo::domain::{
    MIN_SHORT_ID_SUFFIX_LEN, ReferenceError, Space, SpaceId, TaskId, resolve_space_ref,
    resolve_task_ref,
};

#[test]
fn generated_ids_include_prefix_and_short_id() {
    let task_id = TaskId::new();
    let space_id = SpaceId::new();

    assert!(task_id.as_str().starts_with("tsk_"));
    assert!(space_id.as_str().starts_with("spc_"));
    assert_eq!(task_id.short_id().len(), 4 + MIN_SHORT_ID_SUFFIX_LEN);
    assert_eq!(space_id.short_id().len(), 4 + MIN_SHORT_ID_SUFFIX_LEN);
    assert!(task_id.matches_ref(&task_id.short_id()).unwrap());
    assert!(space_id.matches_ref(&space_id.short_id()).unwrap());
}

#[test]
fn ambiguous_short_task_reference_is_rejected() {
    let first = TaskId::try_from("tsk_01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap();
    let second = TaskId::try_from("tsk_01ARZ3NDM7QJ4P9WV7YB9X6T1R").unwrap();

    let error = resolve_task_ref("tsk_01ARZ3ND", [&first, &second]).unwrap_err();
    assert!(matches!(
        error,
        ReferenceError::AmbiguousTaskReference { ref matches, .. } if matches.len() == 2
    ));
}

#[test]
fn space_reference_supports_slug_and_short_id() {
    let mut personal = Space::new("Personal Workspace", 0);
    let mut work = Space::new("Work", 1);
    personal.id = SpaceId::try_from("spc_01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap();
    work.id = SpaceId::try_from("spc_01BX5ZZKBKACTAV9WEVGEMMVRZ").unwrap();

    let resolved_by_slug = resolve_space_ref("personal_workspace", [&personal, &work]).unwrap();
    let resolved_by_short = resolve_space_ref(&work.id.short_id(), [&personal, &work]).unwrap();

    assert_eq!(resolved_by_slug, personal.id);
    assert_eq!(resolved_by_short, work.id);
}
