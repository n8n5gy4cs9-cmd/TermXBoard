# TermXBoard

A colorful terminal dashboard for macOS. Combines a general information dashboard with a live project-task board driven by a `progress.json` file.

![Dashboard](docs/screenshots/dashboard.png)

## Features

- **Dashboard** — large clock/date, weather, computer telemetry (CPU, memory, battery), and developer-news headlines (Hacker News, Simon Willison LLMs, GitHub Blog)
- **Task View** — project-focused view with tasks grouped and filtered from a loaded `progress.json` file
- **7 themes** — Signature Neon, Cyberpunk, Matrix, Nord, Dracula, Solarized Dark, Amber CRT

## Prerequisites

- macOS
- [Rust](https://rustup.rs) 1.96+

## Quick start

```bash
./start.sh
```

This builds the release binary (if needed) and launches the app.

## Build only

```bash
./build.sh
```

The binary is placed at `dist/TermXBoard`.

## Install globally

To run `TermXBoard` from any directory, add the binary to your `PATH`. The easiest way is to symlink into `~/.local/bin` (already on most macOS PATHs):

```bash
ln -sf "$(pwd)/dist/TermXBoard" ~/.local/bin/TermXBoard
```

After this, you can launch it from anywhere:

```bash
TermXBoard
```

If you prefer copying over symlinking (e.g. to keep a stable snapshot):

```bash
cp dist/TermXBoard ~/.local/bin/TermXBoard
OR NOT
copy to:

cp dist/TermXBoard /Users/harriahola/.local/bin/TermXBoard
```

> Re-run `./build.sh` then `cp dist/TermXBoard ~/.local/bin/` after each build if you used `cp`. The symlink approach stays up to date automatically.

## Configuration

`dist/config.json` holds user preferences (theme, remembered project path, etc.). The app auto-creates it on first run if missing.

## Progress file

Task View reads a `progress.json` file that describes your project: milestones, phases, tasks, dependencies, and statuses. See `docs/PROGRESS_JSON_GUIDE.md` for the schema.
