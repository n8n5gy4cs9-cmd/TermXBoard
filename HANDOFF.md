# Handoff — Finish TermXBoard (09 + 10)

For DeepSeek V4 Pro. Continue in this repo; do not rebuild completed work.

## Objective

Implement, verify, review, then mark done:

- `.scratch/termxboard/issues/09-live-project-refresh.md`
- `.scratch/termxboard/issues/10-integrated-polish-and-verification.md`

Read first: `CLAUDE.md`, `CONTEXT.md`, both issue files. Use the local `implement` skill if available. Use TDD at the seams below. KISS/YAGNI.

## Current state

Tickets 01–08 are implemented. Tickets 06–08 are complete but uncommitted because the last Codex sandbox could not create `.git/index.lock`. Their issue files are marked `done`.

Current full gate passes:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
./build.sh
file dist/TermXBoard
test -x dist/TermXBoard
git diff --check
```

`dist/TermXBoard` currently builds as macOS arm64 Mach-O.

Important existing modules:

- `src/progress.rs`: Progress File parsing/validation, `ProjectSession`, load menu, board, selection, filters, projection, grouping, Task controls.
- `src/ui.rs`: event loop and all Dashboard/Task View/modal rendering.
- `src/{telemetry,weather,news}.rs`: nonblocking monitor patterns worth copying.
- `src/preferences.rs`: portable preferences stored beside executable.
- `src/lib.rs`: app/view/key/settings state.

Ticket 08 behavior already works: `f` filters, `g` groups, `Esc` clears, projected navigation, header summary, unknown facets, `Unspecified`, hidden Current Task notice.

## Ticket 09 — recommended design

Add a focused project-refresh monitor, preferably in `progress.rs` or a small `project_refresh.rs` module. Mirror the existing monitor APIs; do not put filesystem timing/change logic directly into rendering.

Suggested seam:

```text
ProjectMonitor
  new(active LoadedProject, 60s interval, now)
  tick(now, task_view_active)
  refresh_now(now)
  view/state -> valid Project + health + updated timestamp + changed Task IDs
```

Required behavior:

- Poll only while Task View is active. Leaving Task View pauses scheduling/work; returning resumes appropriately.
- `r` requests immediate Progress File refresh only. It must not refresh weather/news.
- Always re-read the active file path. Never read `TASKS.md`; never rewrite `progress.json`.
- Parse and validate through the same rules as initial loading; avoid divergent loaders.
- Valid update atomically replaces the Project.
- Invalid/missing/partially-written JSON retains the last valid Project, displays red LED + concise error, then recovers on a later valid read.
- Detect changed Tasks from supplied Task fields. Include added/removed/modified IDs. Do not infer semantic changes.
- Brief highlight needs an explicit expiry and must not mutate Task data. Use one simple duration constant.
- Show last successful update timestamp in Task View. Distinguish loading/error/healthy with existing LED vocabulary: orange/ red/green.
- Preserve current selection when its ID still exists; otherwise choose a visible projected Task. Re-run projection after refresh without clearing active filters/grouping.
- If `currentTask` changes, honor the new supplied value.

Test first:

- no refresh before 60s; refresh at 60s;
- no scheduled refresh on Dashboard; resume in Task View;
- `r` immediate refresh;
- valid changed file updates Project and reports exact changed IDs;
- missing/malformed/partial file retains last good state and exposes error;
- later valid file clears error and recovers;
- added/removed/modified Task detection;
- highlight expiry;
- selection/filter/grouping survival after update.

Avoid blocking UI ticks. Files are local and small, so synchronous reads may be acceptable, but prove the event loop remains responsive or use the established worker/channel pattern if needed. Do not over-engineer watchers, debounce systems, hashes, histories, or persistence.

## Ticket 10 — polish checklist

Finish only what the issue requires. Audit existing behavior before adding abstractions.

### Themes/readability

- Seven existing named presets only: Signature Neon, Cyberpunk, Matrix, Nord, Dracula, Solarized Dark, Amber CRT.
- Exercise Dashboard, Task View, filter/group menus, load/settings/help, details, errors, and resize state in every theme.
- Add compact render tests for palette contrast/readability where practical; manually inspect representative 110×32 frames.
- Do not add custom/light/automatic/per-widget themes.

### Motion/fallbacks

- Add one restrained decorative animation only if absent. `reduced_motion=true` must make it static.
- Never animate essential data or impair input/refresh timing.
- Add readable fallback for terminals without true-color and/or icon support. Prefer a small capability/presentation choice at startup; ANSI/text must remain clear.
- Do not invent a terminal-detection framework. Environment/terminal capability uncertainty must degrade safely.

### Controls/help

Audit every actionable key against runtime and help/footer:

```text
l load, d dashboard, t tasks, s settings, h help, q immediate quit
w weather refresh, n news refresh, r Progress File refresh
f filters, g grouping, Esc clear/cancel, arrows/Enter menu/navigation/open
```

Fully keyboard-driven. `q` still exits immediately outside text-entry modes. RSS remains Dashboard-only; compact top bar remains Task View-only.

### Final verification

- Run the full gate shown above.
- Manual smoke test: `./start.sh`, minimum/undersized resize recovery, all views/modals/themes, load relative + absolute `progress.json`, edit valid/invalid/valid during Task View, 60s refresh, `r`, Dashboard pause, network success/failure/stale states, external headline open, immediate quit.
- Confirm `./build.sh` produces executable under `dist/` and running it keeps config beside executable.
- Perform two-axis review: repo standards (`CLAUDE.md` + `CONTEXT.md`) and exact ticket specs. Fix findings, rerun gate.
- Set each issue to `**Status:** done` and check every box only after verification.

## Non-negotiable domain rules

- `progress.json` is the sole Task source. No `TASKS.md`, issue/spec mining, fabricated fields, or inferred statuses.
- Stored statuses only: `blocked`, `in-progress`, `awaiting-review`, `todo`, `done`. Display labels may differ.
- Dependencies never auto-block a Task.
- Missing `currentTask` stays missing; never choose one semantically.
- Remembered Project is offered, never auto-opened.
- One active Project only.
- Never modify the user's Progress File.
- Dashboard: clock/weather/telemetry/news. Task View: compact clock/date/weather + Tasks; no RSS or telemetry.
- Preserve unknown JSON fields by tolerating them, not displaying invented meaning.
- Clear errors; keep last valid local/network data during transient failures.

## Git/worktree caution

The worktree is intentionally dirty. Preserve unrelated/user-owned files, especially `CLAUDE.md`. `HANDOFF.md` and `dist/` are untracked. Do not reset, discard, or overwrite existing 06–08 work.

Before editing:

```sh
git status --short
git diff --check
```

If Git writes work in your environment, commit coherent feature files only. Baseline before 06–08 was `0717bf0`; review untracked files too because ordinary `git diff` omits them. If `.git/index.lock` is denied again, finish and verify everything, then report the exact blocker—do not pretend a commit exists.

## Definition of done

Tickets 09 and 10 marked done; all acceptance boxes checked; full automated gate green; manual smoke results recorded concisely; review findings resolved; no regressions to 01–08; Git commit made if permissions allow.
