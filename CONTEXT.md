# TermXBoard

TermXBoard is a colorful terminal dashboard for macOS. It combines a general information dashboard with a live project-task board driven by Ralph's `progress.json`.

## Views

**Dashboard**:
The default view: large clock/date, weather, computer telemetry, and developer-news headlines. It remains useful without a loaded project.
_Avoid_: Home screen, task dashboard, project dashboard, RSS panel in Task View, compact top bar

**Task View**:
The project-focused view: compact clock/date/weather bar plus tasks grouped, filtered, and sorted from the loaded Progress File.
_Avoid_: Dashboard, RSS view, TASKS.md view, equal emphasis on system telemetry

## Project progress

**Progress File**:
The selected `progress.json`; sole source of project, milestone, task, dependency, status, and aggregate-progress data. Re-read while Task View is active.
_Avoid_: TASKS.md, spec file, issue file, cached copy, inferred task data, rewriting user data

**Project**:
The project described by the loaded Progress File. Only one Project is active per session.
_Avoid_: Workspace, repository, issue tracker, multiple simultaneous projects

**Task**:
A work item explicitly present in the Progress File. Its ID, title, milestone, phase, dependencies, notes, and status come from that file only.
_Avoid_: Ticket, issue, TODO comment, invented task, task inferred from TASKS.md

**Current Task**:
The Task referenced by `currentTask`. It is emphasized but never assigned or inferred when the field is empty.
_Avoid_: First unfinished task, recommended task, active filter result, auto-selected task

**Milestone**:
A named project stage declared in the Progress File and referenced by Tasks.
_Avoid_: Phase, status group, category invented from task IDs

**Phase**:
A Task classification supplied by the Progress File. It is distinct from Milestone even when values appear related.
_Avoid_: Milestone, inferred sequence, status

**Task Status**:
One of `blocked`, `in-progress`, `awaiting-review`, `todo`, or `done` as supplied by the Progress File.
_Avoid_: WIP as stored value, user-review as stored value, undone as stored value, complete, finished, pending, assumed status

**Status Label**:
The display wording for a Task Status: Blocked, WIP, User Review, Undone, or Done.
_Avoid_: Changing the stored status, treating labels as JSON values, silently mapping unknown values

**Blocked Task**:
A Task whose explicit status is `blocked`. Dependencies provide context; they do not independently change its status.
_Avoid_: Auto-blocked task, unmet dependency equals blocked, hidden task

## News

**News Feed**:
One of the configured developer-news sources shown only on Dashboard: Hacker News, Simon Willison LLMs, or GitHub Blog.
_Avoid_: Social feed, task activity, arbitrary scraped site, news in Task View, invented feed

**Headline**:
A feed-provided title, source, publication time, and URL opened externally for reading.
_Avoid_: Full article, generated summary, rewritten title, fabricated timestamp or URL

## Preferences

**Theme**:
One of seven named color presets: Signature Neon, Cyberpunk, Matrix, Nord, Dracula, Solarized Dark, or Amber CRT.
_Avoid_: User-defined theme, automatic theme, light theme, invented preset, per-widget palette

**Remembered Project**:
The last successfully loaded Progress File path offered on Dashboard. It is not automatically treated as valid or opened.
_Avoid_: Auto-loaded project, project history, recent-project list, guaranteed-existing path
