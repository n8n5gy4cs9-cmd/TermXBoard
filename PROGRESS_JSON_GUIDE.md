# TermXBoard `progress.json` Guide

TermXBoard reads `progress.json` directly. It never reads `TASKS.md`. The file may live anywhere and may be loaded with a relative or absolute path.

## Minimum valid file

```json
{
  "tasks": [
    {
      "id": "T-1",
      "status": "todo"
    }
  ]
}
```

Required:

- Root `tasks` must be an array.
- Every task needs a non-empty, unique string `id`.
- Every task needs exactly one supported `status`.

Supported stored statuses:

| JSON value | TermXBoard label |
|---|---|
| `blocked` | Blocked |
| `in-progress` | WIP |
| `awaiting-review` | User Review |
| `todo` | Undone |
| `done` | Done |

Do not write `wip`, `pending`, `complete`, `finished`, `review`, or `user-review`.

## Recommended complete file

```json
{
  "$schema": "./progress.schema.json",
  "project": "my-project",
  "version": "0.1.0",
  "updatedAt": "2026-08-08T12:00:00Z",
  "currentTask": "T-2",
  "currentPhase": "P1",
  "verifyCommand": "cargo test",
  "definitionOfDone": [
    "Tests pass",
    "Requested behavior is verified"
  ],
  "milestones": [
    {
      "id": "M1",
      "name": "Working foundation",
      "status": "in-progress",
      "exit": "Core workflow works end to end",
      "note": null
    }
  ],
  "tasks": [
    {
      "id": "T-1",
      "title": "Create project shell",
      "status": "done",
      "milestone": "M1",
      "phase": "P0",
      "mode": "Think High",
      "dependsOn": [],
      "scopeNote": "Only the application shell",
      "filesChanged": ["Cargo.toml", "src/main.rs"],
      "notes": "Implemented and verified",
      "verifiedAt": "2026-08-08T11:30:00Z",
      "verification": {
        "command": "cargo test",
        "result": "passed"
      }
    },
    {
      "id": "T-2",
      "title": "Build the dashboard",
      "status": "in-progress",
      "milestone": "M1",
      "phase": "P1",
      "mode": "Think High",
      "dependsOn": ["T-1"],
      "scopeNote": null,
      "filesChanged": [],
      "notes": null,
      "verifiedAt": null,
      "verification": null
    },
    {
      "id": "T-3",
      "title": "Review dashboard output",
      "status": "todo",
      "milestone": "M1",
      "phase": "P1",
      "mode": "Review",
      "dependsOn": ["T-2"],
      "scopeNote": null,
      "filesChanged": [],
      "notes": null,
      "verifiedAt": null,
      "verification": null
    }
  ],
  "followUps": [],
  "notes": []
}
```

`$schema` and unknown fields are allowed but ignored by TermXBoard.

## Field reference

Root fields:

| Field | Type | Rule |
|---|---|---|
| `tasks` | array | Required. Sole Task source. |
| `project` | string or `null` | Project name. |
| `version` | string or `null` | Project/version label. |
| `updatedAt` | string or `null` | Prefer an RFC 3339 UTC timestamp. |
| `currentTask` | string or `null` | Must name the actual current Task, or be `null`. Never guess. |
| `currentPhase` | string or `null` | Current supplied Phase. |
| `verifyCommand` | string or `null` | Command Ralph should run to verify work. |
| `definitionOfDone` | string array | Optional; defaults to `[]`. |
| `milestones` | object array | Optional; defaults to `[]`. Each object requires `id`; `name`, `status`, `exit`, and `note` are optional. |
| `followUps` | array | Optional free-form JSON values; defaults to `[]`. |
| `notes` | array | Optional free-form JSON values; defaults to `[]`. |

Task fields:

| Field | Type | Rule |
|---|---|---|
| `id` | string | Required, non-empty, unique, and stable. |
| `status` | string | Required; use only the five supported values. |
| `title` | string or `null` | Human-readable Task title. |
| `milestone` | string or `null` | Milestone grouping value. Prefer a declared milestone ID. |
| `phase` | string or `null` | Phase grouping value; distinct from Milestone. |
| `mode` | string or `null` | Execution/reasoning mode supplied by the workflow. |
| `dependsOn` | string array | Dependency Task IDs; defaults to `[]`. Dependencies do not automatically make a Task blocked. |
| `scopeNote` | string or `null` | Scope boundary. |
| `filesChanged` | string array | Files actually changed; defaults to `[]`. |
| `notes` | string or `null` | Latest factual Task note. |
| `verifiedAt` | string or `null` | Prefer an RFC 3339 UTC timestamp; set only after verification. |
| `verification` | any JSON or `null` | Factual verification evidence. An object is recommended. |

## Ralph update contract

On every run, Ralph must:

1. Read the existing `progress.json` before choosing work.
2. Treat its Tasks and IDs as authoritative. Do not reconstruct Tasks from `TASKS.md`.
3. Select only a Task whose dependencies and project rules permit work.
4. Set that Task to `in-progress` and set `currentTask` to its ID when work begins.
5. Update `currentPhase` only from the selected Task/project data.
6. Record only files actually changed in `filesChanged`.
7. Run `verifyCommand` or the Task-specific required checks.
8. Set the final status honestly:
   - `done`: implementation and required verification passed.
   - `awaiting-review`: implementation is ready but human/agent review is required.
   - `blocked`: work cannot proceed; explain the real blocker in `notes`.
   - `todo`: work has not begun or was deliberately returned to the queue.
   - `in-progress`: work genuinely remains active.
9. Set `verifiedAt` and `verification` only when checks actually ran. Clear stale verification if later changes invalidate it.
10. Set `updatedAt` to the current RFC 3339 UTC time whenever the file changes.
11. Set `currentTask` to the real active Task, or `null` when none is active. Never select the first unfinished Task merely for display.
12. Preserve unknown fields and unrelated Task data.
13. Write valid JSON atomically: write a temporary file beside `progress.json`, validate it, then rename it over the original. Never expose a partially written file.

TermXBoard refreshes the active file every 60 seconds in Task View. Press `r` for an immediate refresh.

## Validation checklist

Before saving:

- JSON parses without comments or trailing commas.
- `tasks` exists and is an array.
- Every Task has one unique, non-empty `id`.
- Every Task has one exact supported `status`.
- `currentTask` is `null` or matches an existing Task ID.
- Every `dependsOn` entry matches an existing Task ID.
- Array fields are arrays, not strings or `null`.
- No status or verification result was inferred or fabricated.
- The update preserves unrelated and unknown fields.

## Copy-paste AI prompt

```text
Convert the supplied project progress JSON into a TermXBoard-compatible progress.json and update ralph.sh so every Ralph run maintains it correctly.

Authoritative TermXBoard contract:
- progress.json is the sole Task source. Do not read, generate, or synchronize TASKS.md.
- Root `tasks` is required and must be an array.
- Every Task requires a stable, non-empty, unique string `id` and one exact status: `blocked`, `in-progress`, `awaiting-review`, `todo`, or `done`.
- Never store display labels such as WIP, User Review, Undone, or Done as alternative status values.
- Preserve meaningful existing IDs, Tasks, ordering, unknown fields, and factual data. Do not invent milestones, phases, dependencies, completion, verification, timestamps, or current work.
- Root camelCase fields supported by the app: project, version, updatedAt, currentTask, currentPhase, verifyCommand, definitionOfDone, milestones, tasks, followUps, notes.
- Task camelCase fields supported by the app: id, status, title, milestone, phase, mode, dependsOn, scopeNote, filesChanged, notes, verifiedAt, verification.
- Use [] for absent array fields and null for absent optional scalar/object fields in the normalized output.
- currentTask must be null or an existing Task ID. Never infer it from the first unfinished Task.
- Dependencies provide context; they do not automatically change status to blocked.

Update ralph.sh with this run contract:
1. Read the existing progress.json first and use its Tasks as authoritative.
2. When real work begins, set only the selected Task to `in-progress` and set currentTask to its ID.
3. After work, record only actual filesChanged and factual notes.
4. Run verifyCommand/required checks before claiming verification.
5. Use `done` only when required work and verification pass; `awaiting-review` when review is needed; `blocked` only for a real blocker; otherwise retain the honest state.
6. Set verifiedAt and verification only from checks that actually ran; remove stale verification after invalidating changes.
7. Update updatedAt on every mutation and keep currentTask/currentPhase truthful.
8. Preserve unknown fields and unrelated Task data.
9. Serialize valid JSON and replace progress.json atomically using a temporary sibling file plus rename. On validation or write failure, keep the previous file and exit with a clear error.
10. Never rewrite the entire Task plan from another file and never fabricate progress.

Deliver:
1. The converted, valid progress.json.
2. The minimal ralph.sh patch needed to enforce the contract.
3. A short list of source fields that could not be mapped safely and were preserved or set to null.
4. Validation showing JSON parsing succeeded, IDs are unique, statuses are valid, currentTask/dependencies resolve, and ralph.sh syntax checks pass.

Do not add unrelated features or dependencies. If source meaning is ambiguous, preserve it in an unknown field or report it; do not guess.

SOURCE progress.json:
<PASTE THE EXISTING JSON HERE>

SOURCE ralph.sh:
<PASTE THE EXISTING SCRIPT HERE>
```

## Alternative input: Task-Master compatible tasks.json

TermXBoard reads files in the Task-Master format (the shape described in
`prd.schema.json`) from the same load prompt — no flag needed. Point the loader
at the file exactly as you would at a `progress.json`.

Two shapes are supported:

**Tagged** (the common shape, used by Task-Master):

```json
{
  "master": {
    "tasks": [
      { "id": 1, "title": "Bootstrap", "status": "done", "dependencies": [], "subtasks": [] }
    ],
    "metadata": { "updated": "2026-08-01T09:00:00Z", "description": "my-project" }
  }
}
```

**Legacy** (flat root array, useful for simple lists):

```json
{
  "tasks": [
    { "id": "T-1", "title": "Setup", "status": "done", "dependencies": [], "subtasks": [] }
  ]
}
```

A file is treated as Task-Master when **any** of these hold:

- `$schema` ends with `prd.schema.json`
- the root has no `tasks` key but contains a value with a `tasks` array (tagged shape)
- the first task has an integer `id`
- the first task has a `dependencies`, `subtasks`, or `testStrategy` field
- the first task's `status` is `pending`, `review`, `deferred`, or `cancelled`

### Task-Master status mapping

| Task-Master value | TermXBoard label | Notes |
|---|---|---|
| `pending` | Undone | |
| `in-progress` | WIP | |
| `done` | Done | |
| `review` | User Review | |
| `deferred` | Undone | `notes` records `Stored status: deferred` |
| `cancelled` | Done | `notes` records `Stored status: cancelled` |

### Task-Master field mapping

| Task-Master | TermXBoard |
|---|---|
| `id` (int or string) | `id` as string |
| `title` | `title` |
| `description` | `scopeNote` |
| `details`, `testStrategy`, `priority` | folded into `notes` |
| `status` | mapped status (see above) |
| `dependencies` | `dependsOn` (ids coerced to string) |
| `affectedAssets` | `filesChanged` |
| `subtasks` | flattened as tasks with id `<parent>.<sub>` |
| `metadata.updated` | `updatedAt` |
| `metadata.description` | project `project` name |

The first `in-progress` task becomes `currentTask`. Task-Master files carry no
milestone or phase, so those facets are empty.

## Alternative input: Classic PRD Files

TermXBoard also reads a classic PRD File from the same load prompt — no flag,
no separate command. Point the loader at it exactly as you would at a
`progress.json`.

A file is treated as a classic PRD when any of these hold:

- the root has `goal`, `non_goals`, or `stack`
- the first Task has `acceptance`, `paths`, or `verify`

Everything else keeps loading as a native Progress File, unchanged.

### PRD status mapping

| PRD value | TermXBoard label |
|---|---|
| `blocked` | Blocked |
| `in-progress` | WIP |
| `todo` | Undone |
| `done` | Done |
| `error` | Blocked (Task notes start with `Stored status: error`) |

`error` has no canonical TermXBoard status, so it is shown as Blocked and the
stored value is preserved in the Task notes.

### PRD field mapping

Root:

| PRD | TermXBoard |
|---|---|
| `project`, `version`, `updated_at` | same fields |
| `goal`, `non_goals`, `constraints`, `open_questions` | `definitionOfDone`, each line prefixed with its origin |
| `stack`, `hardware_targets`, `task_status_values` | project `notes` entries |
| — | `currentTask` is the first `in-progress` Task; `currentPhase` follows from it |

Task:

| PRD | TermXBoard |
|---|---|
| `id`, `title`, `notes` | same fields |
| `phase` (integer) | `phase` rendered as `P<n>` |
| `deps` | `dependsOn` |
| `paths` | `filesChanged` |
| `acceptance` | `scopeNote`, and appended to `notes` |
| `verify` | `verification` |
| `model_hint` | `mode` |
| `finished_at`, else `started_at` | `verifiedAt` |
| `error_note`, `blocked_question`, `est_minutes` | folded into `notes` |

PRD Tasks carry no milestone, so a PRD board has no Milestone facet. Grouping
and filtering by Status and Phase work as usual.

## Alternative input: User-Story PRD Files

TermXBoard also reads the user-story PRD shape (`schemaVersion`/`currentTask`/
`userStories`) from the same load prompt. A file is treated as a user-story PRD
when the root has a `userStories` array, so it is detected before the classic
PRD rules above.

### User-story status mapping

| PRD value | TermXBoard label |
|---|---|
| `done` | Done |
| `in_progress` | WIP |
| `todo` | Undone |
| `blocked`, `error` | Blocked |

### User-story field mapping

Root:

| PRD | TermXBoard |
|---|---|
| `project`, `updatedAt`, `currentTask` | `project`, `updatedAt`, `currentTask` |
| `description` | `definitionOfDone` as `Description: …` |
| `schemaVersion`, `branchName`, `schemaSource` | project `notes` entries |

Story:

| PRD | TermXBoard |
|---|---|
| `id`, `title` | same fields |
| `status` | mapped status (see above) |
| `dependsOn` | `dependsOn` |
| `description` | `scopeNote`, and first entry of `notes` |
| `acceptanceCriteria` | `notes` as `Acceptance: …` |
| `priority` | `notes` as `Priority: <n>` |
| `passes`, `evidence` | `verification` (`passes` plus a per-criterion `result`/`artifact` summary) |
| `evidence[].artifact` | `filesChanged` |
| `updatedAt` (only when `passes`) | `verifiedAt` |

User-story PRDs carry no milestone or phase, so that board has only the Status
facet plus Status grouping; the Milestone and Phase facets stay empty.
