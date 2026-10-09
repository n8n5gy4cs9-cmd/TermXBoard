use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::Deserialize;

use crate::KeyCommand;

pub const REFRESH_INTERVAL: Duration = Duration::from_secs(60);
const HIGHLIGHT_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Blocked,
    InProgress,
    AwaitingReview,
    Todo,
    Done,
}

impl TaskStatus {
    pub const ORDER: [Self; 5] = [
        Self::Blocked,
        Self::InProgress,
        Self::AwaitingReview,
        Self::Todo,
        Self::Done,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Blocked => "Blocked",
            Self::InProgress => "WIP",
            Self::AwaitingReview => "User Review",
            Self::Todo => "Undone",
            Self::Done => "Done",
        }
    }

    pub fn order_index(self) -> usize {
        Self::ORDER
            .iter()
            .position(|status| *status == self)
            .expect("canonical Task Status")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTask {
    pub id: String,
    pub status: TaskStatus,
    pub title: Option<String>,
    pub milestone: Option<String>,
    pub phase: Option<String>,
    pub mode: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub scope_note: Option<String>,
    #[serde(default)]
    pub files_changed: Vec<String>,
    pub notes: Option<String>,
    pub verified_at: Option<String>,
    pub verification: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub exit: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub project: Option<String>,
    pub version: Option<String>,
    pub updated_at: Option<String>,
    pub current_task: Option<String>,
    pub current_phase: Option<String>,
    pub verify_command: Option<String>,
    #[serde(default)]
    pub definition_of_done: Vec<String>,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    pub tasks: Vec<ProjectTask>,
    #[serde(default)]
    pub follow_ups: Vec<serde_json::Value>,
    #[serde(default)]
    pub notes: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProject {
    pub path: PathBuf,
    pub project: Project,
}

pub fn load_progress_file(
    input_path: impl AsRef<Path>,
    cwd: impl AsRef<Path>,
) -> Result<LoadedProject, String> {
    let input_path = input_path.as_ref();
    let resolved = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        cwd.as_ref().join(input_path)
    };
    let contents = fs::read_to_string(&resolved)
        .map_err(|error| format!("could not read {}: {error}", resolved.display()))?;
    let project = parse_project_document(&contents, &resolved)?;
    validate_project(&project)?;
    let path = fs::canonicalize(&resolved)
        .map_err(|error| format!("could not resolve {}: {error}", resolved.display()))?;
    Ok(LoadedProject { path, project })
}

/// Reads a native Progress File, a classic PRD File, a user-story PRD File,
/// or a Task-Master compatible tasks.json into the Project model.
fn parse_project_document(contents: &str, resolved: &Path) -> Result<Project, String> {
    if crate::prd::looks_like_user_story_prd(contents) {
        return crate::prd::parse_user_story_prd(contents)
            .map_err(|error| describe_parse_error(&error, resolved));
    }
    if crate::prd::looks_like_task_master(contents) {
        return crate::prd::parse_task_master(contents)
            .map_err(|error| describe_parse_error(&error, resolved));
    }
    if crate::prd::looks_like_prd(contents) {
        return crate::prd::parse(contents)
            .map_err(|error| describe_parse_error(&error, resolved));
    }
    serde_json::from_str(contents).map_err(|error| describe_parse_error(&error, resolved))
}

fn describe_parse_error(error: &serde_json::Error, resolved: &Path) -> String {
    let kind = match error.classify() {
        serde_json::error::Category::Data => "missing required data or invalid value",
        _ => "malformed JSON",
    };
    format!("{kind} in {}: {error}", resolved.display())
}

fn validate_project(project: &Project) -> Result<(), String> {
    let mut ids = HashSet::new();
    for task in &project.tasks {
        if task.id.trim().is_empty() {
            return Err("invalid Progress File: Task id cannot be empty".into());
        }
        if !ids.insert(task.id.as_str()) {
            return Err(format!(
                "invalid Progress File: duplicate Task id {}",
                task.id
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSession {
    active: Option<LoadedProject>,
    remembered_project: Option<PathBuf>,
}

impl ProjectSession {
    pub fn new(remembered_project: Option<PathBuf>) -> Self {
        Self {
            active: None,
            remembered_project,
        }
    }

    pub fn active(&self) -> Option<&LoadedProject> {
        self.active.as_ref()
    }

    pub fn remembered_project(&self) -> Option<&Path> {
        self.remembered_project.as_deref()
    }

    pub fn load(
        &mut self,
        input_path: impl AsRef<Path>,
        cwd: impl AsRef<Path>,
    ) -> Result<&LoadedProject, String> {
        let loaded = load_progress_file(input_path, cwd)?;
        self.remembered_project = Some(loaded.path.clone());
        self.active = Some(loaded);
        Ok(self.active.as_ref().expect("just loaded"))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LoadMenuStage {
    #[default]
    Closed,
    Choose,
    PathInput,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadProjectMenu {
    stage: LoadMenuStage,
    remembered_project: Option<PathBuf>,
    remembered_selected: bool,
    path_draft: String,
}

impl LoadProjectMenu {
    pub fn open(&mut self, remembered_project: Option<PathBuf>) {
        self.stage = LoadMenuStage::Choose;
        self.remembered_project = remembered_project;
        self.remembered_selected = false;
        self.path_draft.clear();
    }

    pub fn stage(&self) -> LoadMenuStage {
        self.stage
    }

    pub fn path_draft(&self) -> &str {
        &self.path_draft
    }

    pub fn remembered_project(&self) -> Option<&Path> {
        self.remembered_project.as_deref()
    }

    pub fn remembered_selected(&self) -> bool {
        self.remembered_selected
    }

    pub fn handle_key(&mut self, key: KeyCommand) -> Option<PathBuf> {
        if key == KeyCommand::Escape {
            self.stage = LoadMenuStage::Closed;
            return None;
        }
        match self.stage {
            LoadMenuStage::Closed => {}
            LoadMenuStage::Choose => match key {
                KeyCommand::Up | KeyCommand::Down if self.remembered_project.is_some() => {
                    self.remembered_selected = !self.remembered_selected;
                }
                KeyCommand::Enter if self.remembered_selected => {
                    self.stage = LoadMenuStage::Closed;
                    return self.remembered_project.clone();
                }
                KeyCommand::Enter => self.stage = LoadMenuStage::PathInput,
                _ => {}
            },
            LoadMenuStage::PathInput => match key {
                KeyCommand::Character(character) => self.path_draft.push(character),
                KeyCommand::Backspace => {
                    self.path_draft.pop();
                }
                KeyCommand::Enter if !self.path_draft.trim().is_empty() => {
                    self.stage = LoadMenuStage::Closed;
                    return Some(PathBuf::from(self.path_draft.trim()));
                }
                _ => {}
            },
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGroup<'a> {
    pub status: TaskStatus,
    pub label: &'static str,
    pub tasks: Vec<&'a ProjectTask>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyStatus<'a> {
    pub id: &'a str,
    pub status: Option<TaskStatus>,
}

pub struct TaskBoard<'a> {
    project: &'a Project,
}

impl<'a> TaskBoard<'a> {
    pub fn new(project: &'a Project) -> Self {
        Self { project }
    }

    pub fn groups(&self) -> Vec<TaskGroup<'a>> {
        TaskStatus::ORDER
            .into_iter()
            .map(|status| TaskGroup {
                status,
                label: status.label(),
                tasks: self
                    .project
                    .tasks
                    .iter()
                    .filter(|task| task.status == status)
                    .collect(),
            })
            .collect()
    }

    pub fn ordered_tasks(&self) -> Vec<&'a ProjectTask> {
        self.groups()
            .into_iter()
            .flat_map(|group| group.tasks)
            .collect()
    }

    pub fn dependencies(&self, task: &'a ProjectTask) -> Vec<DependencyStatus<'a>> {
        task.depends_on
            .iter()
            .map(|id| DependencyStatus {
                id,
                status: self
                    .project
                    .tasks
                    .iter()
                    .find(|candidate| candidate.id == *id)
                    .map(|dependency| dependency.status),
            })
            .collect()
    }

    pub fn is_current(&self, task: &ProjectTask) -> bool {
        self.project.current_task.as_deref() == Some(task.id.as_str())
    }

    pub fn current_task_notice<I, S>(&self, visible_task_ids: I) -> Option<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let current = self.project.current_task.as_deref()?;
        if !self.project.tasks.iter().any(|task| task.id == current) {
            return Some(format!(
                "Current Task {current} is not present in Progress File"
            ));
        }
        if visible_task_ids
            .into_iter()
            .any(|visible| visible.as_ref() == current)
        {
            None
        } else {
            Some(format!("Current Task {current} hidden by active filters"))
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskSelection {
    task_id: Option<String>,
}

impl TaskSelection {
    pub fn for_project(project: &Project) -> Self {
        let board = TaskBoard::new(project);
        let task_id = project
            .current_task
            .as_ref()
            .filter(|id| project.tasks.iter().any(|task| task.id == id.as_str()))
            .cloned()
            .or_else(|| board.ordered_tasks().first().map(|task| task.id.clone()));
        Self { task_id }
    }

    pub fn selected<'a>(&self, project: &'a Project) -> Option<&'a ProjectTask> {
        let selected = self.task_id.as_deref()?;
        project.tasks.iter().find(|task| task.id == selected)
    }

    pub fn is_selected(&self, task: &ProjectTask) -> bool {
        self.task_id.as_deref() == Some(task.id.as_str())
    }

    pub fn next(&mut self, project: &Project) {
        self.move_by(project, 1);
    }

    pub fn previous(&mut self, project: &Project) {
        self.move_by(project, -1);
    }

    pub fn next_in(&mut self, tasks: &[&ProjectTask]) {
        self.move_within(tasks, 1);
    }

    pub fn previous_in(&mut self, tasks: &[&ProjectTask]) {
        self.move_within(tasks, -1);
    }

    pub fn ensure_visible(&mut self, tasks: &[&ProjectTask]) {
        if self
            .task_id
            .as_deref()
            .is_none_or(|id| !tasks.iter().any(|task| task.id == id))
        {
            self.task_id = tasks.first().map(|task| task.id.clone());
        }
    }

    fn move_by(&mut self, project: &Project, offset: isize) {
        let tasks = TaskBoard::new(project).ordered_tasks();
        self.move_within(&tasks, offset);
    }

    fn move_within(&mut self, tasks: &[&ProjectTask], offset: isize) {
        if tasks.is_empty() {
            self.task_id = None;
            return;
        }
        let current = self
            .task_id
            .as_deref()
            .and_then(|id| tasks.iter().position(|task| task.id == id))
            .unwrap_or(0);
        let next = current
            .saturating_add_signed(offset)
            .min(tasks.len().saturating_sub(1));
        self.task_id = Some(tasks[next].id.clone());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FacetValue {
    Supplied(String),
    Unspecified,
}

impl FacetValue {
    pub fn from_optional(value: Option<&str>) -> Self {
        value
            .map(|value| Self::Supplied(value.to_string()))
            .unwrap_or(Self::Unspecified)
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Supplied(value) => value,
            Self::Unspecified => "Unspecified",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskFilters {
    statuses: BTreeSet<TaskStatus>,
    milestones: BTreeSet<FacetValue>,
    phases: BTreeSet<FacetValue>,
}

impl TaskFilters {
    pub fn toggle_status(&mut self, status: TaskStatus) {
        toggle_set_value(&mut self.statuses, status);
    }

    pub fn toggle_milestone(&mut self, milestone: FacetValue) {
        toggle_set_value(&mut self.milestones, milestone);
    }

    pub fn toggle_phase(&mut self, phase: FacetValue) {
        toggle_set_value(&mut self.phases, phase);
    }

    pub fn clear(&mut self) {
        self.statuses.clear();
        self.milestones.clear();
        self.phases.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.statuses.is_empty() && self.milestones.is_empty() && self.phases.is_empty()
    }

    pub fn matches(&self, task: &ProjectTask) -> bool {
        (self.statuses.is_empty() || self.statuses.contains(&task.status))
            && (self.milestones.is_empty()
                || self
                    .milestones
                    .contains(&FacetValue::from_optional(task.milestone.as_deref())))
            && (self.phases.is_empty()
                || self
                    .phases
                    .contains(&FacetValue::from_optional(task.phase.as_deref())))
    }

    pub fn summary(&self) -> String {
        let mut categories = Vec::new();
        if !self.statuses.is_empty() {
            categories.push(format!(
                "Status={}",
                self.statuses
                    .iter()
                    .map(|status| status.label())
                    .collect::<Vec<_>>()
                    .join("|")
            ));
        }
        if !self.milestones.is_empty() {
            categories.push(format!(
                "Milestone={}",
                self.milestones
                    .iter()
                    .map(FacetValue::label)
                    .collect::<Vec<_>>()
                    .join("|")
            ));
        }
        if !self.phases.is_empty() {
            categories.push(format!(
                "Phase={}",
                self.phases
                    .iter()
                    .map(FacetValue::label)
                    .collect::<Vec<_>>()
                    .join("|")
            ));
        }
        categories.join("  ")
    }
}

fn toggle_set_value<T: Ord>(set: &mut BTreeSet<T>, value: T) {
    if !set.remove(&value) {
        set.insert(value);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GroupBy {
    #[default]
    Status,
    Milestone,
    Phase,
}

impl GroupBy {
    pub const ALL: [Self; 3] = [Self::Status, Self::Milestone, Self::Phase];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Status => "Status",
            Self::Milestone => "Milestone",
            Self::Phase => "Phase",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedTaskGroup<'a> {
    pub label: String,
    pub tasks: Vec<&'a ProjectTask>,
}

pub struct TaskProjection<'a> {
    groups: Vec<ProjectedTaskGroup<'a>>,
    visible_task_ids: Vec<&'a str>,
    current_task_notice: Option<String>,
}

impl<'a> TaskProjection<'a> {
    pub fn new(project: &'a Project, filters: &TaskFilters, group_by: GroupBy) -> Self {
        let visible = project
            .tasks
            .iter()
            .filter(|task| filters.matches(task))
            .collect::<Vec<_>>();
        let visible_task_ids = visible
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();
        let current_task_notice =
            TaskBoard::new(project).current_task_notice(visible_task_ids.iter().copied());
        let groups = match group_by {
            GroupBy::Status => TaskStatus::ORDER
                .into_iter()
                .filter_map(|status| {
                    let tasks = visible
                        .iter()
                        .copied()
                        .filter(|task| task.status == status)
                        .collect::<Vec<_>>();
                    (!tasks.is_empty()).then(|| ProjectedTaskGroup {
                        label: status.label().to_string(),
                        tasks,
                    })
                })
                .collect(),
            GroupBy::Milestone => group_by_facet(visible, |task| task.milestone.as_deref()),
            GroupBy::Phase => group_by_facet(visible, |task| task.phase.as_deref()),
        };
        Self {
            groups,
            visible_task_ids,
            current_task_notice,
        }
    }

    pub fn groups(&self) -> &[ProjectedTaskGroup<'a>] {
        &self.groups
    }

    pub fn visible_task_ids(&self) -> &[&'a str] {
        &self.visible_task_ids
    }

    pub fn current_task_notice(&self) -> Option<&str> {
        self.current_task_notice.as_deref()
    }

    pub fn ordered_tasks(&self) -> Vec<&'a ProjectTask> {
        self.groups
            .iter()
            .flat_map(|group| group.tasks.iter().copied())
            .collect()
    }
}

fn group_by_facet<'a>(
    tasks: Vec<&'a ProjectTask>,
    value: impl Fn(&ProjectTask) -> Option<&str>,
) -> Vec<ProjectedTaskGroup<'a>> {
    let mut grouped: BTreeMap<FacetValue, Vec<&ProjectTask>> = BTreeMap::new();
    for task in tasks {
        grouped
            .entry(FacetValue::from_optional(value(task)))
            .or_default()
            .push(task);
    }
    grouped
        .into_iter()
        .map(|(facet, mut tasks)| {
            tasks.sort_by_key(|task| task.status.order_index());
            ProjectedTaskGroup {
                label: facet.label().to_string(),
                tasks,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TaskMenu {
    #[default]
    Closed,
    Filters,
    Grouping,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FilterCategory {
    #[default]
    Status,
    Milestone,
    Phase,
}

impl FilterCategory {
    const ALL: [Self; 3] = [Self::Status, Self::Milestone, Self::Phase];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Status => "Status",
            Self::Milestone => "Milestone",
            Self::Phase => "Phase",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterOption {
    pub label: String,
    pub selected: bool,
}

enum FilterChoice {
    Status(TaskStatus),
    Milestone(FacetValue),
    Phase(FacetValue),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskControls {
    filters: TaskFilters,
    group_by: GroupBy,
    menu: TaskMenu,
    filter_category_index: usize,
    filter_option_index: usize,
    grouping_index: usize,
}

impl TaskControls {
    pub fn filters(&self) -> &TaskFilters {
        &self.filters
    }

    pub fn group_by(&self) -> GroupBy {
        self.group_by
    }

    pub fn menu(&self) -> TaskMenu {
        self.menu
    }

    pub fn filter_category(&self) -> FilterCategory {
        FilterCategory::ALL[self.filter_category_index]
    }

    pub fn filter_option_index(&self) -> usize {
        self.filter_option_index
    }

    pub fn grouping_index(&self) -> usize {
        self.grouping_index
    }

    pub fn filter_summary(&self) -> String {
        self.filters.summary()
    }

    pub fn filter_options(&self, project: &Project) -> Vec<FilterOption> {
        self.filter_choices(project)
            .into_iter()
            .map(|choice| match choice {
                FilterChoice::Status(status) => FilterOption {
                    label: status.label().to_string(),
                    selected: self.filters.statuses.contains(&status),
                },
                FilterChoice::Milestone(value) => FilterOption {
                    label: value.label().to_string(),
                    selected: self.filters.milestones.contains(&value),
                },
                FilterChoice::Phase(value) => FilterOption {
                    label: value.label().to_string(),
                    selected: self.filters.phases.contains(&value),
                },
            })
            .collect()
    }

    pub fn handle_key(&mut self, key: KeyCommand, project: &Project) {
        match key {
            KeyCommand::Character('f') => {
                self.menu = if self.menu == TaskMenu::Filters {
                    TaskMenu::Closed
                } else {
                    TaskMenu::Filters
                };
                self.filter_option_index = 0;
                return;
            }
            KeyCommand::Character('g') => {
                self.menu = if self.menu == TaskMenu::Grouping {
                    TaskMenu::Closed
                } else {
                    self.grouping_index = GroupBy::ALL
                        .iter()
                        .position(|group| *group == self.group_by)
                        .unwrap_or(0);
                    TaskMenu::Grouping
                };
                return;
            }
            KeyCommand::Escape => {
                self.menu = TaskMenu::Closed;
                return;
            }
            KeyCommand::Character('c') if self.menu == TaskMenu::Filters => {
                self.filters.clear();
                return;
            }
            _ => {}
        }

        match self.menu {
            TaskMenu::Closed => {}
            TaskMenu::Filters => self.handle_filter_key(key, project),
            TaskMenu::Grouping => self.handle_grouping_key(key),
        }
    }

    fn handle_filter_key(&mut self, key: KeyCommand, project: &Project) {
        match key {
            KeyCommand::Left => {
                self.filter_category_index = (self.filter_category_index + 2) % 3;
                self.filter_option_index = 0;
            }
            KeyCommand::Right => {
                self.filter_category_index = (self.filter_category_index + 1) % 3;
                self.filter_option_index = 0;
            }
            KeyCommand::Up | KeyCommand::Down => {
                let length = self.filter_choices(project).len();
                if length > 0 {
                    self.filter_option_index = if key == KeyCommand::Up {
                        (self.filter_option_index + length - 1) % length
                    } else {
                        (self.filter_option_index + 1) % length
                    };
                }
            }
            KeyCommand::Enter => {
                if let Some(choice) = self
                    .filter_choices(project)
                    .into_iter()
                    .nth(self.filter_option_index)
                {
                    match choice {
                        FilterChoice::Status(status) => self.filters.toggle_status(status),
                        FilterChoice::Milestone(value) => self.filters.toggle_milestone(value),
                        FilterChoice::Phase(value) => self.filters.toggle_phase(value),
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_grouping_key(&mut self, key: KeyCommand) {
        match key {
            KeyCommand::Up => {
                self.grouping_index =
                    (self.grouping_index + GroupBy::ALL.len() - 1) % GroupBy::ALL.len();
            }
            KeyCommand::Down => {
                self.grouping_index = (self.grouping_index + 1) % GroupBy::ALL.len();
            }
            KeyCommand::Enter => {
                self.group_by = GroupBy::ALL[self.grouping_index];
                self.menu = TaskMenu::Closed;
            }
            _ => {}
        }
    }

    fn filter_choices(&self, project: &Project) -> Vec<FilterChoice> {
        match self.filter_category() {
            FilterCategory::Status => TaskStatus::ORDER
                .into_iter()
                .map(FilterChoice::Status)
                .collect(),
            FilterCategory::Milestone => facet_choices(project, |task| task.milestone.as_deref())
                .into_iter()
                .map(FilterChoice::Milestone)
                .collect(),
            FilterCategory::Phase => facet_choices(project, |task| task.phase.as_deref())
                .into_iter()
                .map(FilterChoice::Phase)
                .collect(),
        }
    }
}

fn facet_choices(
    project: &Project,
    value: impl Fn(&ProjectTask) -> Option<&str>,
) -> Vec<FacetValue> {
    project
        .tasks
        .iter()
        .map(|task| FacetValue::from_optional(value(task)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectHealth {
    Healthy,
    Recovered,
    Loading,
    Error { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// User-visible refresh activity at a supplied instant.
pub enum ProjectActivity {
    Healthy,
    Recovered,
    Updating,
    Countdown(u64),
    Error,
}

pub struct ProjectMonitor {
    last_good: Option<LoadedProject>,
    file_path: PathBuf,
    interval: Duration,
    next_refresh: Instant,
    health: ProjectHealth,
    last_update: Option<Instant>,
    changed_task_ids: Vec<String>,
    highlight_until: Option<Instant>,
    updating_until: Option<Instant>,
}

impl ProjectMonitor {
    pub fn new(loaded: &LoadedProject, interval: Duration, now: Instant) -> Self {
        Self {
            last_good: Some(loaded.clone()),
            file_path: loaded.path.clone(),
            interval,
            next_refresh: now + interval,
            health: ProjectHealth::Healthy,
            last_update: Some(now),
            changed_task_ids: Vec::new(),
            highlight_until: None,
            updating_until: None,
        }
    }

    pub fn active(&self) -> Option<&LoadedProject> {
        self.last_good.as_ref()
    }

    pub fn health(&self) -> &ProjectHealth {
        &self.health
    }

    pub fn last_update(&self) -> Option<Instant> {
        self.last_update
    }

    pub fn changed_task_ids(&self) -> &[String] {
        &self.changed_task_ids
    }

    pub fn highlight_until(&self) -> Option<Instant> {
        self.highlight_until
    }

    pub fn error_message(&self) -> Option<&str> {
        match &self.health {
            ProjectHealth::Error { message } => Some(message),
            _ => None,
        }
    }

    /// Returns countdown, feedback, recovery, or error presentation state.
    pub fn activity_at(&self, now: Instant) -> ProjectActivity {
        if matches!(self.health, ProjectHealth::Error { .. }) {
            return ProjectActivity::Error;
        }
        if self.updating_until.is_some_and(|until| now < until) {
            return ProjectActivity::Updating;
        }
        let remaining = self.next_refresh.saturating_duration_since(now);
        if !remaining.is_zero() && remaining <= Duration::from_secs(10) {
            let seconds = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
            return ProjectActivity::Countdown(seconds.max(1));
        }
        if matches!(self.health, ProjectHealth::Recovered) {
            ProjectActivity::Recovered
        } else {
            ProjectActivity::Healthy
        }
    }

    pub fn reset(&mut self, loaded: &LoadedProject, now: Instant) {
        self.last_good = Some(loaded.clone());
        self.file_path = loaded.path.clone();
        self.next_refresh = now + self.interval;
        self.health = ProjectHealth::Healthy;
        self.last_update = Some(now);
        self.changed_task_ids.clear();
        self.highlight_until = None;
        self.updating_until = None;
    }

    pub fn tick(&mut self, now: Instant, task_view_active: bool) {
        if self.highlight_until.is_some_and(|until| now >= until) {
            self.changed_task_ids.clear();
            self.highlight_until = None;
        }
        if !task_view_active || now < self.next_refresh {
            return;
        }
        self.perform_refresh(now);
    }

    pub fn refresh_now(&mut self, now: Instant) {
        self.perform_refresh(now);
    }

    fn perform_refresh(&mut self, now: Instant) {
        self.next_refresh = now + self.interval;
        self.updating_until = Some(now + Duration::from_millis(600));
        let recovering = matches!(self.health, ProjectHealth::Error { .. });
        self.health = ProjectHealth::Loading;
        match load_progress_file(&self.file_path, "") {
            Ok(loaded) => {
                let changed = self
                    .last_good
                    .as_ref()
                    .map(|previous| detect_changes(&previous.project.tasks, &loaded.project.tasks))
                    .unwrap_or_default();
                self.last_good = Some(loaded);
                self.last_update = Some(now);
                self.health = if recovering {
                    ProjectHealth::Recovered
                } else {
                    ProjectHealth::Healthy
                };
                if !changed.is_empty() {
                    self.changed_task_ids = changed;
                    self.highlight_until = Some(now + HIGHLIGHT_DURATION);
                }
            }
            Err(message) => {
                self.health = ProjectHealth::Error { message };
                self.updating_until = None;
            }
        }
    }
}

fn detect_changes(old_tasks: &[ProjectTask], new_tasks: &[ProjectTask]) -> Vec<String> {
    let old_ids: HashSet<&str> = old_tasks.iter().map(|t| t.id.as_str()).collect();
    let new_ids: HashSet<&str> = new_tasks.iter().map(|t| t.id.as_str()).collect();
    let mut changed = Vec::new();
    for id in old_ids.difference(&new_ids) {
        changed.push(id.to_string());
    }
    for id in new_ids.difference(&old_ids) {
        changed.push(id.to_string());
    }
    for new_task in new_tasks {
        if old_ids.contains(new_task.id.as_str())
            && let Some(old_task) = old_tasks.iter().find(|t| t.id == new_task.id)
            && old_task != new_task
        {
            changed.push(new_task.id.clone());
        }
    }
    changed
}
