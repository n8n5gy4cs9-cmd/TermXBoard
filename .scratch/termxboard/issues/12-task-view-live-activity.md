# 12 — Task View live activity

**What to build:** Make Task View visibly alive and operationally clear without confusing explicit WIP state, keyboard selection, Current Task, refresh feedback, or errors.

**Blocked by:** 09 — Live project refresh; 11 — Retro-terminal visual overhaul.

**Status:** done

- [x] Highlight every Task whose supplied status is `in-progress`, under Status, Milestone, and Phase grouping. Use a synchronized 1.2-second cyan breathing edge plus persistent `[WIP]` badge; never infer a WIP Task.
- [x] Keep state layers visually independent at minimum width: first line `┃▌> ▶ T-123`, second line `[CURRENT] [WIP] Title`. `>` is the selection cursor in every glyph mode with an inverse row background; `┃`/`[CURRENT]` identify Current Task; `▌`/`[WIP]` and the WIP status icon identify explicit WIP. Each applicable layer remains visible when combined.
- [x] Show `HH:MM:SS` in the compact Task View top bar and update it every second. Do not change Dashboard clock behavior or layout.
- [x] Keep the 60-second Progress File schedule. During only its final ten seconds show a ceiling-rounded `NEXT UPDATE 10s` through `NEXT UPDATE 1s`; never show `0s`. Returning from Dashboard after an elapsed deadline refreshes immediately.
- [x] Show `UPDATING` for at least 600 ms after scheduled or manual `r` refresh begins, while applying valid data immediately; start the next 60-second interval when the read begins, not when feedback ends. An error replaces `UPDATING` immediately.
- [x] Add a centered 24-cell Knight Rider scanner at the far right of the third Task View header line: bright three-cell head, dim one-cell tail, bouncing at about eight frames per second. Use normal Theme accent when healthy/recovered, faster amber during countdown/updating, and static red during errors.
- [x] Add a small health heartbeat beside Task View health: slow green when healthy/recovered, amber during the final countdown, fast amber while updating, and steady red on error.
- [x] On error, show `● ERROR` plus a cleaned single-line summary in the remaining third-line header space when it fits, always reserving the scanner's 24 cells. Collapse whitespace/newlines and remove redundant parser/path noise without hiding the core failure. Show the full wrapped error as the first red section in Task Details above the selected Task's normal details.
- [x] Reduced motion freezes the scanner at center, makes WIP edges steady cyan, and makes the heartbeat steady in its current state color. Informational clock seconds and countdown continue updating.
- [x] Add deterministic, supplied-time tests for WIP/selection/Current layering, multiple synchronized WIP Tasks, grouping independence, `HH:MM:SS`, `10→1` ceiling countdown, Dashboard pause/resume, manual and scheduled refresh, minimum 600 ms updating feedback, scanner bounce/state colors, heartbeat states, cleaned/full errors, ASCII/Nerd Font fallbacks, and reduced motion. Do not use real sleeps.
