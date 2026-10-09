# 11 — Retro-terminal visual overhaul

**What to build:** Replace the flat, sparse presentation with a balanced-dense retro command-center visual system inspired by Mole's scannable icon language and eDEX-UI's cohesive instrumentation, while preserving TermXBoard's keyboard-first usability and honest data.

**Blocked by:** 09 — Live project refresh.

**Status:** done

- [x] Apply one coherent component system across Dashboard, Task View, help, settings, load/filter/group menus, details, loading, empty, error, recovery, and resize states; use stronger hierarchy, purposeful spacing, decorated titles/corners, varied border emphasis, a compact TermXBoard wordmark, and one repeated data-grid/tick motif.
- [x] Redesign Dashboard geometry into a balanced-dense command center: prominent clock/identity, compact telemetry instruments, weather, project action/state, and clearly separated developer news; exploit larger terminals through flexible sizing and extra rows while remaining composed at 110×32.
- [x] Make Task state immediately scannable without relying on color alone: `◆` red Blocked, `▶` cyan WIP, `◉` magenta User Review, `○` amber Undone, and `✓` green Done; adapt shades to each Theme, use row highlighting for selection, and add an accent rail plus `CURRENT` badge for Current Task.
- [x] Enrich Task rows with icon, ID, title, and responsive dependency/milestone/phase metadata; hide optional metadata when width is insufficient rather than clipping essential identity or status.
- [x] Add CPU and memory history sparklines only; do not expand TermXBoard into a general monitoring suite.
- [x] Improve weather with condition icons, emphasized numeric temperature, and compact humidity/wind bars without forecasts; color temperature as cold ≤5°C, mild 6–20°C, warm 21–27°C, and hot ≥28°C while retaining text/icon meaning.
- [x] Give each configured news source a consistent icon/accent and give the selected headline a strong focus row; do not alter, summarize, or invent feed content.
- [x] Give every asynchronous state a designed, concise treatment: orange loading scan, red fault frame/message, dim empty state, green healthy/recovered indication, and the ticket-09 changed-Task highlight.
- [x] Add restrained animation only for an activity pulse, CPU/memory sparklines, loading scan, and brief changed-Task glow; make every decorative effect static when reduced motion is enabled. No blinking text, moving Task rows, sound, or mouse behavior.
- [x] Use Unicode/block graphics by default. Add a persisted Nerd Font setting beside existing Theme/reduced-motion preferences, default off, and a complete ASCII fallback that preserves layout and meaning (`X`, `>`, `?`, `o`, `v`, `|`) when enhanced glyphs or true color are unavailable.
- [x] Keep exactly the seven named Themes and one shared layout/component system; verify semantic colors, focus, text, borders, errors, and fallbacks remain readable in every Theme. Do not create custom, light, automatic, or per-widget Themes.
- [x] Prefer existing Ratatui capabilities; allow at most one lightweight maintained dependency only when Ratatui cannot reasonably provide the feature, with its necessity justified in review.
- [x] Add automated render/readability tests for all Themes, status treatments, focus/current layering, reduced motion, Nerd Font mode, ASCII/ANSI fallback, state visuals, responsive minimum layout, and essential text preservation.
- [x] Commit four stable, secret-free 110×32 reference captures using deterministic fixture data: `docs/screenshots/dashboard-signature-neon.png`, `docs/screenshots/tasks-signature-neon.png`, `docs/screenshots/dashboard-amber-crt.png`, and `docs/screenshots/tasks-amber-crt.png`.

**Research references:** [Mole](https://github.com/tw93/mole) for compact semantic icons; [eDEX-UI](https://github.com/GitSquared/edex-ui) for cohesive command-center composition; [Lazygit](https://github.com/jesseduffield/lazygit) and the [Ratatui showcase](https://ratatui.rs/) for scannable state, focus, and terminal-native layout. Use as principles, not pixel-copy targets.
