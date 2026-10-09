//! PRD Files: alternative Progress File schemas.
//!
//! TermXBoard keeps reading its native `progress.json` unchanged. A file that
//! follows either PRD schema is recognised on load and adapted into the same
//! [`Project`] the rest of the application already renders:
//!
//! - the classic PRD schema (`project`/`goal`/`stack`/`tasks` with `paths`,
//!   `acceptance` and `verify` on every Task);
//! - the user-story PRD schema (`project`/`schemaVersion`/`currentTask`/
//!   `userStories` with `acceptanceCriteria`, `passes`, `status`, `dependsOn`
//!   and `evidence` on every Story).

use serde::Deserialize;

use crate::progress::{Project, ProjectTask, TaskStatus};

/// Stored Task status values a PRD File may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrdTaskStatus {
    Todo,
    InProgress,
    Done,
    Error,
    Blocked,
}

impl PrdTaskStatus {
    /// The canonical TermXBoard status this PRD status is shown as.
    ///
    /// `error` has no canonical counterpart, so it is shown as Blocked and the
    /// original value is kept in the Task notes.
    pub const fn as_task_status(self) -> TaskStatus {
        match self {
            Self::Todo => TaskStatus::Todo,
            Self::InProgress => TaskStatus::InProgress,
            Self::Done => TaskStatus::Done,
            Self::Error | Self::Blocked => TaskStatus::Blocked,
        }
    }

    pub const fn stored_label(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in-progress",
            Self::Done => "done",
            Self::Error => "error",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PrdTask {
    pub id: String,
    pub status: PrdTaskStatus,
    pub title: Option<String>,
    pub phase: Option<i64>,
    #[serde(default)]
    pub deps: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub acceptance: Vec<String>,
    pub verify: Option<String>,
    pub est_minutes: Option<i64>,
    pub model_hint: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error_note: Option<String>,
    pub blocked_question: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Prd {
    pub project: Option<String>,
    pub version: Option<String>,
    pub updated_at: Option<String>,
    pub goal: Option<String>,
    #[serde(default)]
    pub non_goals: Vec<String>,
    #[serde(default)]
    pub hardware_targets: Vec<serde_json::Value>,
    pub stack: Option<serde_json::Value>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
    #[serde(default)]
    pub task_status_values: Vec<String>,
    pub tasks: Vec<PrdTask>,
}

/// Reports whether the supplied document is a PRD File rather than a native
/// Progress File. Anything that is not recognisably a PRD stays on the native
/// path, including text that is not JSON at all.
pub fn looks_like_prd(contents: &str) -> bool {
    let Ok(document) = serde_json::from_str::<serde_json::Value>(contents) else {
        return false;
    };
    let Some(root) = document.as_object() else {
        return false;
    };
    if root.contains_key("goal") || root.contains_key("non_goals") || root.contains_key("stack") {
        return true;
    }
    root.get("tasks")
        .and_then(serde_json::Value::as_array)
        .and_then(|tasks| tasks.first())
        .and_then(serde_json::Value::as_object)
        .is_some_and(|task| {
            task.contains_key("acceptance")
                || task.contains_key("paths")
                || task.contains_key("verify")
        })
}

/// Reads a PRD File and adapts it into the Progress File model.
pub fn parse(contents: &str) -> Result<Project, serde_json::Error> {
    Ok(serde_json::from_str::<Prd>(contents)?.into_project())
}

impl Prd {
    /// Adapts this PRD into the Project the dashboard renders.
    pub fn into_project(self) -> Project {
        let current_task = self
            .tasks
            .iter()
            .find(|task| task.status == PrdTaskStatus::InProgress)
            .map(|task| task.id.clone());
        let current_phase = current_task
            .as_ref()
            .and_then(|id| self.tasks.iter().find(|task| task.id == *id))
            .and_then(|task| task.phase)
            .map(phase_label);
        let mut definition_of_done = Vec::new();
        if let Some(goal) = &self.goal {
            definition_of_done.push(format!("Goal: {goal}"));
        }
        for non_goal in &self.non_goals {
            definition_of_done.push(format!("Non-goal: {non_goal}"));
        }
        for constraint in &self.constraints {
            definition_of_done.push(format!("Constraint: {constraint}"));
        }
        for question in &self.open_questions {
            definition_of_done.push(format!("Open question: {question}"));
        }
        let mut notes = Vec::new();
        if let Some(stack) = self.stack {
            notes.push(serde_json::json!({ "stack": stack }));
        }
        let hardware_targets = self.hardware_targets;
        if !hardware_targets.is_empty() {
            notes.push(serde_json::json!({ "hardware_targets": hardware_targets }));
        }
        let task_status_values = self.task_status_values;
        if !task_status_values.is_empty() {
            notes.push(serde_json::json!({ "task_status_values": task_status_values }));
        }
        Project {
            project: self.project,
            version: self.version,
            updated_at: self.updated_at,
            current_task,
            current_phase,
            verify_command: None,
            definition_of_done,
            milestones: Vec::new(),
            tasks: self
                .tasks
                .into_iter()
                .map(PrdTask::into_project_task)
                .collect(),
            follow_ups: Vec::new(),
            notes,
        }
    }
}

impl PrdTask {
    /// Adapts this PRD Task into the Task the board and details pane render.
    pub fn into_project_task(self) -> ProjectTask {
        ProjectTask {
            status: self.status.as_task_status(),
            milestone: None,
            phase: self.phase.map(phase_label),
            mode: self.model_hint.clone(),
            depends_on: self.deps.clone(),
            scope_note: (!self.acceptance.is_empty()).then(|| self.acceptance.join("; ")),
            files_changed: self.paths.clone(),
            notes: self.notes_summary(),
            verified_at: self.finished_at.clone().or_else(|| self.started_at.clone()),
            verification: self
                .verify
                .clone()
                .map(|verify| serde_json::json!({ "verify": verify })),
            id: self.id,
            title: self.title,
        }
    }

    /// Folds the PRD-only Task fields into the notes the details pane shows.
    fn notes_summary(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.status == PrdTaskStatus::Error {
            parts.push(format!("Stored status: {}", self.status.stored_label()));
        }
        if let Some(error_note) = trimmed(self.error_note.as_deref()) {
            parts.push(format!("Error: {error_note}"));
        }
        if let Some(question) = trimmed(self.blocked_question.as_deref()) {
            parts.push(format!("Blocked question: {question}"));
        }
        if let Some(notes) = trimmed(self.notes.as_deref()) {
            parts.push(notes.to_string());
        }
        if !self.acceptance.is_empty() {
            parts.push(format!("Acceptance: {}", self.acceptance.join("; ")));
        }
        if let Some(estimate) = self.est_minutes {
            parts.push(format!("Estimate: {estimate} min"));
        }
        (!parts.is_empty()).then(|| parts.join(" | "))
    }
}

fn trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn phase_label(phase: i64) -> String {
    format!("P{phase}")
}

/// Stored Story status values a user-story PRD File may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStoryStatus {
    Done,
    InProgress,
    Todo,
    Blocked,
    Error,
}

impl UserStoryStatus {
    /// The canonical TermXBoard status this Story status is shown as.
    pub const fn as_task_status(self) -> TaskStatus {
        match self {
            Self::Done => TaskStatus::Done,
            Self::InProgress => TaskStatus::InProgress,
            Self::Todo => TaskStatus::Todo,
            Self::Blocked | Self::Error => TaskStatus::Blocked,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UserStoryEvidence {
    pub criterion: Option<i64>,
    pub result: Option<String>,
    pub detail: Option<String>,
    pub artifact: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStory {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    pub priority: Option<i64>,
    #[serde(default)]
    pub passes: bool,
    pub notes: Option<String>,
    pub status: UserStoryStatus,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub evidence: Vec<UserStoryEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStoryPrd {
    pub project: Option<String>,
    pub description: Option<String>,
    pub schema_version: Option<i64>,
    pub branch_name: Option<String>,
    pub schema_source: Option<String>,
    pub updated_at: Option<String>,
    pub current_task: Option<String>,
    #[serde(default)]
    pub user_stories: Vec<UserStory>,
}

/// Reports whether the supplied document is a user-story PRD File (a root
/// `userStories` array) rather than a native Progress File or a classic PRD.
pub fn looks_like_user_story_prd(contents: &str) -> bool {
    let Ok(document) = serde_json::from_str::<serde_json::Value>(contents) else {
        return false;
    };
    document
        .as_object()
        .and_then(|root| root.get("userStories"))
        .and_then(serde_json::Value::as_array)
        .is_some()
}

/// Reads a user-story PRD File and adapts it into the Progress File model.
pub fn parse_user_story_prd(contents: &str) -> Result<Project, serde_json::Error> {
    Ok(serde_json::from_str::<UserStoryPrd>(contents)?.into_project())
}

impl UserStoryPrd {
    /// Adapts this user-story PRD into the Project the dashboard renders.
    pub fn into_project(self) -> Project {
        let mut definition_of_done = Vec::new();
        if let Some(description) = trimmed(self.description.as_deref()) {
            definition_of_done.push(format!("Description: {description}"));
        }
        let mut notes = Vec::new();
        if let Some(schema_version) = self.schema_version {
            notes.push(serde_json::json!({ "schemaVersion": schema_version }));
        }
        if let Some(branch_name) = trimmed(self.branch_name.as_deref()) {
            notes.push(serde_json::json!({ "branchName": branch_name }));
        }
        if let Some(schema_source) = trimmed(self.schema_source.as_deref()) {
            notes.push(serde_json::json!({ "schemaSource": schema_source }));
        }
        Project {
            project: self.project,
            version: None,
            updated_at: self.updated_at,
            current_task: self.current_task,
            current_phase: None,
            verify_command: None,
            definition_of_done,
            milestones: Vec::new(),
            tasks: self
                .user_stories
                .into_iter()
                .map(UserStory::into_project_task)
                .collect(),
            follow_ups: Vec::new(),
            notes,
        }
    }
}

impl UserStory {
    /// Adapts this Story into the Task the board and details pane render.
    pub fn into_project_task(self) -> ProjectTask {
        let mut notes_parts = Vec::new();
        if let Some(description) = trimmed(self.description.as_deref()) {
            notes_parts.push(description.to_string());
        }
        if let Some(notes) = trimmed(self.notes.as_deref()) {
            notes_parts.push(notes.to_string());
        }
        if !self.acceptance_criteria.is_empty() {
            notes_parts.push(format!(
                "Acceptance: {}",
                self.acceptance_criteria.join("; ")
            ));
        }
        if let Some(priority) = self.priority {
            notes_parts.push(format!("Priority: {priority}"));
        }
        let notes = (!notes_parts.is_empty()).then(|| notes_parts.join(" | "));
        let files_changed = self
            .evidence
            .iter()
            .filter_map(|entry| trimmed(entry.artifact.as_deref()).map(str::to_string))
            .collect();
        let verification = (self.passes || !self.evidence.is_empty()).then(|| {
            serde_json::json!({
                "passes": self.passes,
                "evidence": self
                    .evidence
                    .iter()
                    .map(evidence_summary)
                    .collect::<Vec<_>>(),
            })
        });
        ProjectTask {
            status: self.status.as_task_status(),
            milestone: None,
            phase: None,
            mode: None,
            depends_on: self.depends_on,
            scope_note: self.description,
            files_changed,
            notes,
            verified_at: if self.passes {
                self.updated_at.clone()
            } else {
                None
            },
            verification,
            id: self.id,
            title: self.title,
        }
    }
}

fn evidence_summary(entry: &UserStoryEvidence) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    if let Some(criterion) = entry.criterion {
        map.insert("criterion".to_string(), serde_json::json!(criterion));
    }
    if let Some(result) = entry.result.as_deref() {
        map.insert("result".to_string(), serde_json::json!(result));
    }
    if let Some(artifact) = entry.artifact.as_deref() {
        map.insert("artifact".to_string(), serde_json::json!(artifact));
    }
    serde_json::Value::Object(map)
}

// ── Task-Master format ────────────────────────────────────────────────────────
//
// Two shapes are supported:
//   legacy  { "tasks": [...] }
//   tagged  { "<tag>": { "tasks": [...], "metadata": {} } }
//
// Status mapping: pending/deferred → todo, in-progress → in-progress,
//                 done → done, review → awaiting-review, cancelled → done
// Integer IDs are coerced to strings. Subtasks are flattened with
// composite IDs ("<parent>.<sub>") so they appear in the task board.

/// Status values used by Task-Master compatible task files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskMasterStatus {
    Pending,
    InProgress,
    Done,
    Review,
    Deferred,
    Cancelled,
}

impl TaskMasterStatus {
    pub const fn as_task_status(self) -> TaskStatus {
        match self {
            Self::Pending | Self::Deferred => TaskStatus::Todo,
            Self::InProgress => TaskStatus::InProgress,
            Self::Done | Self::Cancelled => TaskStatus::Done,
            Self::Review => TaskStatus::AwaitingReview,
        }
    }

    pub const fn stored_label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in-progress",
            Self::Done => "done",
            Self::Review => "review",
            Self::Deferred => "deferred",
            Self::Cancelled => "cancelled",
        }
    }

    /// Returns true for terminal-but-not-done statuses that need a note.
    pub const fn needs_status_note(self) -> bool {
        matches!(self, Self::Deferred | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMasterTask {
    pub id: serde_json::Value,
    pub title: Option<String>,
    pub description: Option<String>,
    pub details: Option<String>,
    pub test_strategy: Option<String>,
    pub status: Option<TaskMasterStatus>,
    pub priority: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<serde_json::Value>,
    #[serde(default)]
    pub affected_assets: Vec<String>,
    #[serde(default)]
    pub subtasks: Vec<TaskMasterTask>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskMasterMetadata {
    pub updated: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskMasterSection {
    pub tasks: Vec<TaskMasterTask>,
    pub metadata: Option<TaskMasterMetadata>,
}

/// Normalises a Task-Master ID (integer or string) to a stable string.
fn tm_id(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Reports whether the document matches the Task-Master tasks.json shape
/// (legacy `{ tasks: [] }` or tagged `{ <tag>: { tasks: [] } }`).
pub fn looks_like_task_master(contents: &str) -> bool {
    let Ok(document) = serde_json::from_str::<serde_json::Value>(contents) else {
        return false;
    };
    let Some(root) = document.as_object() else {
        return false;
    };

    // Explicit schema reference to the Task-Master schema file.
    if root
        .get("$schema")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|s| s.ends_with("prd.schema.json"))
    {
        return true;
    }

    // Tagged shape: no top-level "tasks" key, but at least one value is an
    // object that itself has a "tasks" array.
    if !root.contains_key("tasks") && !root.contains_key("userStories") {
        let tagged = root.values().any(|v| {
            v.as_object()
                .and_then(|obj| obj.get("tasks"))
                .and_then(serde_json::Value::as_array)
                .is_some()
        });
        if tagged {
            return true;
        }
    }

    // Legacy shape: has top-level "tasks" with Task-Master markers.
    if let Some(first_task) = root
        .get("tasks")
        .and_then(serde_json::Value::as_array)
        .and_then(|a| a.first())
        .and_then(serde_json::Value::as_object)
    {
        if first_task
            .get("id")
            .is_some_and(|v| v.is_number())
        {
            return true;
        }
        if first_task.contains_key("dependencies")
            || first_task.contains_key("subtasks")
            || first_task.contains_key("testStrategy")
        {
            return true;
        }
        if first_task
            .get("status")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| matches!(s, "pending" | "review" | "deferred" | "cancelled"))
        {
            return true;
        }
    }

    false
}

/// Parses a Task-Master tasks.json into the Progress File model.
pub fn parse_task_master(contents: &str) -> Result<Project, serde_json::Error> {
    let document: serde_json::Value = serde_json::from_str(contents)?;
    let root = document.as_object().unwrap();

    // Detect shape.
    let (tasks, metadata) = if root.contains_key("tasks") {
        // Legacy: { tasks: [...] }
        let tasks: Vec<TaskMasterTask> =
            serde_json::from_value(root["tasks"].clone())?;
        (tasks, None)
    } else {
        // Tagged: { "<tag>": { tasks: [...], metadata: {} } }
        // Use whichever tag comes first (usually "master").
        let section_value = root
            .values()
            .find(|v| {
                v.as_object()
                    .and_then(|o| o.get("tasks"))
                    .and_then(serde_json::Value::as_array)
                    .is_some()
            })
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let section: TaskMasterSection = serde_json::from_value(section_value)?;
        let meta = section.metadata.clone();
        (section.tasks, meta)
    };

    // Collect all tasks (top-level + flattened subtasks).
    let mut project_tasks: Vec<ProjectTask> = Vec::new();
    let mut current_task: Option<String> = None;
    let mut updated_at: Option<String> = metadata.as_ref().and_then(|m| m.updated.clone());
    let project_name: Option<String> =
        metadata.as_ref().and_then(|m| m.description.clone());

    for task in &tasks {
        let task_id = tm_id(&task.id);
        let dep_ids: Vec<String> = task.dependencies.iter().map(tm_id).collect();
        let pt = tm_task_to_project_task(task, &task_id, dep_ids, None);

        // Track first in-progress task as current.
        if current_task.is_none() && pt.status == TaskStatus::InProgress {
            current_task = Some(task_id.clone());
        }
        if let Some(ts) = &pt.verified_at {
            if updated_at.is_none() {
                updated_at = Some(ts.clone());
            }
        }
        project_tasks.push(pt);

        // Flatten subtasks.
        for sub in &task.subtasks {
            let sub_id = format!("{task_id}.{}", tm_id(&sub.id));
            let sub_deps: Vec<String> = sub.dependencies.iter().map(tm_id).collect();
            let sub_pt = tm_task_to_project_task(sub, &sub_id, sub_deps, Some(&task_id));
            project_tasks.push(sub_pt);
        }
    }

    Ok(Project {
        project: project_name,
        version: None,
        updated_at,
        current_task,
        current_phase: None,
        verify_command: None,
        definition_of_done: Vec::new(),
        milestones: Vec::new(),
        tasks: project_tasks,
        follow_ups: Vec::new(),
        notes: Vec::new(),
    })
}

fn tm_task_to_project_task(
    task: &TaskMasterTask,
    id: &str,
    dep_ids: Vec<String>,
    parent_id: Option<&str>,
) -> ProjectTask {
    let status = task
        .status
        .unwrap_or(TaskMasterStatus::Pending)
        .as_task_status();

    let mut notes_parts: Vec<String> = Vec::new();
    if let Some(stored) = task.status.filter(|s| s.needs_status_note()) {
        notes_parts.push(format!("Stored status: {}", stored.stored_label()));
    }
    if let Some(details) = trimmed(task.details.as_deref()) {
        notes_parts.push(details.to_string());
    }
    if let Some(strategy) = trimmed(task.test_strategy.as_deref()) {
        notes_parts.push(format!("Test strategy: {strategy}"));
    }
    if let Some(priority) = trimmed(task.priority.as_deref()) {
        notes_parts.push(format!("Priority: {priority}"));
    }
    if let Some(parent) = parent_id {
        notes_parts.push(format!("Subtask of: {parent}"));
    }
    let notes = (!notes_parts.is_empty()).then(|| notes_parts.join(" | "));

    ProjectTask {
        id: id.to_string(),
        title: task.title.clone(),
        status,
        milestone: None,
        phase: None,
        mode: None,
        depends_on: dep_ids,
        scope_note: task.description.clone(),
        files_changed: task.affected_assets.clone(),
        notes,
        verified_at: None,
        verification: None,
    }
}
