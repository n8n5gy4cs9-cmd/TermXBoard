use termxboard::progress::{FacetValue, GroupBy, Project, TaskFilters, TaskProjection, TaskStatus};

fn project() -> Project {
    serde_json::from_str(
        r#"{
      "currentTask":"T-REVIEW",
      "tasks":[
        {"id":"T-BLOCKED","status":"blocked","milestone":"M2","phase":"P1"},
        {"id":"T-WIP","status":"in-progress","milestone":"M1","phase":"P1"},
        {"id":"T-REVIEW","status":"awaiting-review","milestone":"M1","phase":"P2"},
        {"id":"T-TODO","status":"todo","milestone":"M2","phase":"P2"},
        {"id":"T-DONE","status":"done","milestone":"Custom milestone"},
        {"id":"T-UNSPECIFIED","status":"todo"}
      ]
    }"#,
    )
    .expect("project fixture")
}

#[test]
fn filters_use_or_within_categories_and_and_across_categories() {
    let project = project();
    let mut filters = TaskFilters::default();
    filters.toggle_status(TaskStatus::InProgress);
    filters.toggle_status(TaskStatus::AwaitingReview);
    filters.toggle_milestone(FacetValue::Supplied("M1".into()));
    filters.toggle_phase(FacetValue::Supplied("P2".into()));

    let projection = TaskProjection::new(&project, &filters, GroupBy::Status);

    assert_eq!(projection.visible_task_ids(), ["T-REVIEW"]);
    assert_eq!(projection.current_task_notice(), None);
}

#[test]
fn grouping_preserves_unknown_names_and_puts_unspecified_last() {
    let project = project();
    let projection = TaskProjection::new(&project, &TaskFilters::default(), GroupBy::Milestone);

    assert_eq!(
        projection
            .groups()
            .iter()
            .map(|group| group.label.as_str())
            .collect::<Vec<_>>(),
        ["Custom milestone", "M1", "M2", "Unspecified"]
    );
    assert_eq!(
        projection.groups()[1]
            .tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        ["T-WIP", "T-REVIEW"]
    );
}

#[test]
fn every_grouping_keeps_the_agreed_status_first_task_order() {
    let project = project();
    let status = TaskProjection::new(&project, &TaskFilters::default(), GroupBy::Status);
    assert_eq!(
        status
            .ordered_tasks()
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        [
            "T-BLOCKED",
            "T-WIP",
            "T-REVIEW",
            "T-TODO",
            "T-UNSPECIFIED",
            "T-DONE"
        ]
    );

    let phase = TaskProjection::new(&project, &TaskFilters::default(), GroupBy::Phase);
    let p2 = phase
        .groups()
        .iter()
        .find(|group| group.label == "P2")
        .expect("P2 group");
    assert_eq!(
        p2.tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        ["T-REVIEW", "T-TODO"]
    );
}

#[test]
fn filtering_current_task_reports_it_hidden_and_clear_restores_all_tasks() {
    let project = project();
    let mut filters = TaskFilters::default();
    filters.toggle_status(TaskStatus::Done);
    let filtered = TaskProjection::new(&project, &filters, GroupBy::Phase);

    assert_eq!(
        filtered.current_task_notice(),
        Some("Current Task T-REVIEW hidden by active filters")
    );

    filters.clear();
    let restored = TaskProjection::new(&project, &filters, GroupBy::Status);
    assert_eq!(restored.visible_task_ids().len(), project.tasks.len());
}
