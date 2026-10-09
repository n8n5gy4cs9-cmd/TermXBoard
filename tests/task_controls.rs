use termxboard::{
    KeyCommand,
    progress::{GroupBy, Project, TaskControls, TaskMenu, TaskProjection},
};

fn project() -> Project {
    serde_json::from_str(
        r#"{
      "tasks":[
        {"id":"T-BLOCKED","status":"blocked","milestone":"M2","phase":"P1"},
        {"id":"T-WIP","status":"in-progress","milestone":"M1","phase":"P1"},
        {"id":"T-CUSTOM","status":"todo","milestone":"Custom","phase":"Experimental"},
        {"id":"T-UNSPECIFIED","status":"done"}
      ]
    }"#,
    )
    .expect("project fixture")
}

#[test]
fn escape_closes_filter_menu_and_c_clears_filters_only_while_it_is_open() {
    let project = project();
    let mut controls = TaskControls::default();

    controls.handle_key(KeyCommand::Character('f'), &project);
    assert_eq!(controls.menu(), TaskMenu::Filters);
    controls.handle_key(KeyCommand::Enter, &project); // Blocked
    controls.handle_key(KeyCommand::Down, &project);
    controls.handle_key(KeyCommand::Enter, &project); // WIP
    controls.handle_key(KeyCommand::Right, &project); // Milestone
    controls.handle_key(KeyCommand::Down, &project); // M1 after Custom
    controls.handle_key(KeyCommand::Enter, &project);

    let visible = TaskProjection::new(&project, controls.filters(), controls.group_by());
    assert_eq!(visible.visible_task_ids(), ["T-WIP"]);
    assert!(controls.filter_summary().contains("Status=Blocked|WIP"));
    assert!(controls.filter_summary().contains("Milestone=M1"));

    controls.handle_key(KeyCommand::Escape, &project);
    assert_eq!(controls.menu(), TaskMenu::Closed);
    assert!(!controls.filters().is_empty());

    controls.handle_key(KeyCommand::Character('c'), &project);
    assert!(!controls.filters().is_empty());

    controls.handle_key(KeyCommand::Character('f'), &project);
    controls.handle_key(KeyCommand::Character('c'), &project);
    assert!(controls.filters().is_empty());
    assert_eq!(controls.menu(), TaskMenu::Filters);
}

#[test]
fn grouping_menu_selects_status_milestone_or_phase() {
    let project = project();
    let mut controls = TaskControls::default();

    controls.handle_key(KeyCommand::Character('g'), &project);
    controls.handle_key(KeyCommand::Down, &project);
    controls.handle_key(KeyCommand::Enter, &project);
    assert_eq!(controls.group_by(), GroupBy::Milestone);

    controls.handle_key(KeyCommand::Character('g'), &project);
    controls.handle_key(KeyCommand::Down, &project);
    controls.handle_key(KeyCommand::Enter, &project);
    assert_eq!(controls.group_by(), GroupBy::Phase);
}

#[test]
fn filter_options_preserve_supplied_unknown_values_and_unspecified() {
    let project = project();
    let mut controls = TaskControls::default();
    controls.handle_key(KeyCommand::Character('f'), &project);
    controls.handle_key(KeyCommand::Right, &project);
    controls.handle_key(KeyCommand::Right, &project);

    let labels = controls
        .filter_options(&project)
        .into_iter()
        .map(|option| option.label)
        .collect::<Vec<_>>();

    assert_eq!(labels, ["Experimental", "P1", "Unspecified"]);
}
