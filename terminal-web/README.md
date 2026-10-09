# TermXBoard Web

Pure **HTML + JS** retro command deck. No Node, no Electron, no build, no server. One file: [`index.html`](index.html).

Double-click it. Full guide: **[GUIDE.md](GUIDE.md)**.

## Two views

- **DASHBOARD** — live clock, date, weather (open-meteo, no key) and news feeds (Hacker News native + optional RSS).
- **TASKS** — kanban board that reads **two** formats, auto-detected:
  - TermXBoard **`progress.json`** — schema: [`progress.schema.json`](progress.schema.json)
  - **`tasks.json` / PRD** (Task-Master) — schema: [`prd.schema.json`](prd.schema.json)

## Read-only & safe

The app **never writes to your JSON**. Files are opened read-only and only ever read; "reload" just re-reads. Only its own settings are saved (to `localStorage`). See [GUIDE.md §1](GUIDE.md).

## Load

Click **LOAD JSON** (`l`) or **drag & drop** a file. Samples: [`sample-progress.json`](sample-progress.json), [`sample-tasks.json`](sample-tasks.json). Chrome/Edge keep a live handle so `r` re-reads the same file (+ optional auto-reload).

## Style

14 themes (Nord, Dracula, Tokyo Night, Catppuccin, Gruvbox, Synthwave '84, …) with CRT glow / scanlines / vignette / font / corners — all persisted.

## Keyboard

`d`/`t` view · `←→↑↓` navigate · `l` load · `r` reload · `g` group · `f` filter · `/` search · `[`/`]` theme · `s` settings. Full table in [GUIDE.md](GUIDE.md).
