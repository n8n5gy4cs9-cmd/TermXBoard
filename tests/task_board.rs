use termxboard::progress::{Project, TaskBoard, TaskSelection, TaskStatus};

fn project() -> Project {
    serde_json::from_str(r#"{
      "project":"Nodus",
      "currentTask":"T-WIP",
      "verifyCommand":"pnpm verify",
      "tasks":[
        {"id":"T-DONE","title":"Done task","status":"done","dependsOn":[],"verifiedAt":"2026-08-06T09:00:00Z"},
        {"id":"T-TODO","title":"Todo task","status":"todo","dependsOn":[]},
        {"id":"T-REVIEW","title":"Review task","status":"awaiting-review","dependsOn":["T-DONE"],"verification":{"tests":"pass"}},
        {"id":"T-WIP","title":"WIP task","status":"in-progress","dependsOn":["T-DONE"],"notes":"Only supplied notes"},
        {"id":"T-BLOCKED","title":"Blocked task","status":"blocked","dependsOn":["T-TODO"]}
      ]
    }"#).expect("project fixture")
}

#[test]
fn task_selection_starts_at_current_task_and_reaches_non_current_details() {
    let project = project();
    let mut selection = TaskSelection::for_project(&project);
    assert_eq!(selection.selected(&project).expect("current").id, "T-WIP");

    selection.previous(&project);
    assert_eq!(
        selection.selected(&project).expect("previous").id,
        "T-BLOCKED"
    );
    selection.next(&project);
    assert_eq!(selection.selected(&project).expect("next").id, "T-WIP");
}

#[test]
fn task_board_uses_canonical_status_order_and_labels() {
    let project = project();
    let board = TaskBoard::new(&project);
    let groups = board.groups();

    assert_eq!(
        groups.iter().map(|group| group.status).collect::<Vec<_>>(),
        TaskStatus::ORDER
    );
    assert_eq!(
        groups.iter().map(|group| group.label).collect::<Vec<_>>(),
        ["Blocked", "WIP", "User Review", "Undone", "Done"]
    );
    assert_eq!(groups[0].tasks[0].id, "T-BLOCKED");
}

#[test]
fn dependency_statuses_and_supplied_verification_are_exposed_without_inference() {
    let project = project();
    let board = TaskBoard::new(&project);
    let review = project
        .tasks
        .iter()
        .find(|task| task.id == "T-REVIEW")
        .expect("task");

    let dependencies = board.dependencies(review);

    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].id, "T-DONE");
    assert_eq!(dependencies[0].status, Some(TaskStatus::Done));
    assert_eq!(
        review.verification.as_ref().expect("verification")["tests"],
        "pass"
    );
    assert_eq!(project.verify_command.as_deref(), Some("pnpm verify"));
}

#[test]
fn current_task_is_emphasized_or_reported_hidden() {
    let project = project();
    let board = TaskBoard::new(&project);

    assert!(board.is_current(&project.tasks[3]));
    assert_eq!(
        board.current_task_notice(["T-DONE", "T-TODO"]),
        Some("Current Task T-WIP hidden by active filters".into())
    );
    assert_eq!(board.current_task_notice(["T-WIP"]), None);
}
