# TermXBoard Web — Command Deck Guide

A single-file, **pure HTML + JS** retro command deck. No Node, no Electron, no build step, no server, no install. Everything lives in [`index.html`](index.html).

It has two views:

- **DASHBOARD** — live clock, date, weather and news feeds (like the terminal app's dashboard).
- **TASKS** — a kanban board of a task file. It reads **two** JSON formats:
  - TermXBoard **`progress.json`**
  - **`tasks.json` / PRD** (the widely-used Task-Master AI-project format)

---

## 1. Is it safe? Will it change my JSON?

**No. The app is strictly read-only. It never writes to your files.**

- Files are opened with the browser's file picker in **read mode only** (`showOpenFilePicker()` defaults to read; write permission is never requested).
- The code only ever calls `getFile()` and reads the text. There is **no** `createWritable()`, no `write()`, no upload — nowhere in the code can it modify `progress.json`, `tasks.json`, or anything else on disk.
- "Reload" and "Auto-reload" simply **re-read** the same file. They cannot alter it.

The only thing the app writes is **its own settings**, into your browser's `localStorage` (theme, effects, city, feeds…). It never touches your task files.

---

## 2. Run it

Double-click [`index.html`](index.html), or drag it into any modern browser. That's it.

Everything works from a `file://` URL. The only features that need the internet are **weather** and **news** (external APIs); the task board and all styling work fully offline.

---

## 3. Load a task file

- Click **LOAD JSON** (or press `l`) and pick any file from anywhere on disk, **or**
- **Drag & drop** the file onto the window.

**Live re-reading:** In Chrome/Edge the file is opened with a *live handle*, so pressing `r` (RELOAD) re-reads the same file, and you can turn on **Auto-reload** in Settings (5–60 s). In Firefox/Safari the browser gives a static snapshot — press `l` again to re-pick for a fresh read (the File System Access API is Chromium-only).

Two ready-made samples are included: [`sample-progress.json`](sample-progress.json) and [`sample-tasks.json`](sample-tasks.json).

The format is **auto-detected**. Invalid files are rejected with a message in the status bar; nothing is loaded and nothing is changed.

---

## 4. The task board

- **Group** by Status / Milestone / Phase / Priority / Mode.
- **Filter** by any status, or "Active (not done)".
- **Search** across id, title, description, files, priority.
- Click a card (or arrow-key to it) to see full detail + a global progress bar.
- The **current task** is highlighted with `◈`.
- For PRD files, **subtasks** appear as indented child cards (`2.1`, `2.2`, …) and a **Tag** selector appears if the file has multiple tags.

### Keyboard

| Key | Action | Key | Action |
|---|---|---|---|
| `d` | Dashboard view | `t` | Tasks view |
| `←` `→` | move column | `↑` `↓` | move card |
| `l` | load file | `r` | reload / refresh dashboard |
| `g` | cycle group | `f` | cycle filter |
| `/` | focus search | `[` `]` | previous / next theme |
| `s` | settings | `Esc` | close settings |

Mouse works everywhere: click cards, use the dropdowns, sliders and theme swatches.

---

## 5. Supported task formats

### 5a. `progress.json` (TermXBoard)

Root `tasks` array. Every task needs a unique string `id` and one of these statuses:

`blocked` · `in-progress` · `awaiting-review` · `todo` · `done`

Minimum valid file:

```json
{ "tasks": [ { "id": "T-1", "status": "todo" } ] }
```

Full field reference: see [`../PROGRESS_JSON_GUIDE.md`](../PROGRESS_JSON_GUIDE.md).
JSON Schema: [`progress.schema.json`](progress.schema.json).

Fields the board uses: `project`, `version`, `updatedAt`, `currentTask`, `verifyCommand`, and per task `id`, `status`, `title`, `milestone`, `phase`, `mode`, `dependsOn`, `scopeNote`, `filesChanged`, `notes`, `verifiedAt`, `verification`.

### 5b. `tasks.json` / PRD (Task-Master compatible)

This is the de-facto JSON format for AI-driven, PRD-generated task backlogs (from `claude-task-master`). Two shapes are accepted:

**Legacy:**

```json
{ "tasks": [ { "id": 1, "title": "…", "status": "pending" } ] }
```

**Tagged (current):**

```json
{
  "master": {
    "tasks": [ { "id": 1, "title": "…", "status": "pending" } ],
    "metadata": { "created": "…", "updated": "…", "description": "my-project" }
  }
}
```

Task fields: `id` (number or string), `title`, `description`, `details`, `testStrategy`, `status`, `priority`, `dependencies` (array of ids), `subtasks` (array of tasks). Optional `affectedAssets` maps to the Files list.

Statuses: `pending` · `in-progress` · `done` · `review` · `deferred` · `cancelled`

JSON Schema: [`prd.schema.json`](prd.schema.json).

**How PRD fields map onto the board:** each status becomes a column; `priority` shows as a colored chip; `dependencies` show with their target's status; `subtasks` become indented child cards (`<id>.<subid>`); `details` → Details row; `testStrategy` → Test row; `description` → Description/Notes. `currentTask` isn't part of the format, so the first `in-progress` task is highlighted.

> Note: "Matt Pocock's PRD" workflow (`/to-prd`) outputs a **Markdown** PRD / GitHub issue, not a JSON schema. The JSON format above is the widely-used Task-Master `tasks.json`, which is the standard machine-readable PRD-task file in AI projects. If your PRD JSON differs, the schema files show exactly what shape the deck expects.

---

## 6. Dashboard

- **Clock / date / weekday** — local, live, updates every second.
- **Weather** — from [open-meteo.com](https://open-meteo.com) (free, **no API key, no tracking**). Set a **City** in Settings (geocoded automatically) or click **Use my location**. Choose °C/km-h or °F/mph.
- **News** — a column per feed. Refresh with `r`, auto-refreshes every 15 minutes.

### News & the CORS reality (important)

A browser cannot fetch arbitrary RSS feeds directly — the sites don't send CORS headers, so the browser blocks them. This is a browser security rule, not a bug.

- **Hacker News** uses its **official JSON API** and needs **no proxy** — it always works.
- **Other RSS feeds** (Simon Willison, GitHub Blog, anything you add) are routed through a **CORS proxy** URL you set in Settings (default `https://api.allorigins.win/raw?url=`). Public proxies are third-party and can be slow or flaky. If a feed says *offline*, the proxy failed — try again, switch proxies, or **clear the proxy field to disable RSS entirely** and keep just Hacker News.

Add/remove feeds and edit the proxy in **Settings → News feeds**.

---

## 7. Themes & style

14 themes, switch with `[` / `]` or in Settings: Signature Neon, Cyberpunk, Matrix, **Nord**, Dracula, Solarized Dark, Amber CRT, Gruvbox, Tokyo Night, Catppuccin Mocha, Monokai, Synthwave '84, One Dark, Ayu Mirage.

Also adjustable: text glow (on/off + strength), scanlines (on/off + depth), vignette, font size, rounded corners.

**Everything persists in `localStorage`** — your theme, effects, weather city, feeds and view are remembered next time.

---

## 8. Troubleshooting

| Symptom | Cause / fix |
|---|---|
| "Unrecognized file" | Not a `tasks` array and not a tagged `{master:{tasks:[]}}`. Check against the schemas. |
| Board loads but a status is wrong | Use only the exact status strings for that format (section 5). |
| `r` says "Static file — re-pick" | You're in Firefox/Safari; re-open with `l` for a fresh read. Live handles are Chromium-only. |
| Weather blank | Set a City (or Use my location) in Settings. |
| A feed says *offline* | CORS proxy failed. Retry, change proxy, or clear it. Hacker News still works. |
| Nothing saved between sessions | `localStorage` disabled (private mode / blocked). |
