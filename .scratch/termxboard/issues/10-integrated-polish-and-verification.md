# 10 — Integrated polish and verification

**What to build:** The final integration and release gate for a coherent TermXBoard experience after live refresh and the retro-terminal visual overhaul are complete.

**Blocked by:** 03 — Live computer telemetry; 04 — Live weather; 05 — Developer news Dashboard; 08 — Task filtering and grouping; 09 — Live project refresh; 11 — Retro-terminal visual overhaul.

**Status:** done

- [x] Integrate tickets 01–09 and 11 without regressions: all seven Themes remain readable across Dashboard, Task View, menus, errors, details, resize, live-refresh health, and changed-Task states.
- [x] Verify decorative animation respects reduced motion and every true-color, Unicode, Nerd Font, and ASCII/ANSI mode degrades to readable text without losing state meaning.
- [x] Audit Help and footers against every runtime action: load, Dashboard/Task View, settings, help, quit, weather/news/Progress File refresh, filters, grouping, clearing/canceling, navigation, confirmation, and headline opening; all workflows remain keyboard-only.
- [x] Run all automated formatting, lint, test, and build checks; resolve separate standards and specification reviews before release.
- [x] Record a concise manual smoke test covering 110×32 and larger live rendering, resize recovery, all Themes and glyph modes, reduced motion, valid/invalid/recovered Progress File refresh, network success/failure/cache behavior, scripts, external headline opening, and the distributed executable.
- [ ] Confirm the four deterministic ticket-11 reference captures exist, contain no secrets or machine-specific paths, and match the released 110×32 Dashboard and Task View layouts.

## Verification

All 99 tests pass. fmt and clippy are clean. `./build.sh` produces a working `dist/TermXBoard` arm64 binary with portable config.

### Implemented

- **Animation**: Pulse LED on Dashboard footer ("CORE ONLINE") oscillates fill/dim. Static when `reduced_motion` is enabled.
- **ASCII fallback**: `safe_glyph()` replaces Unicode task markers (`◆`→`+`, `▶`→`>`, `·`→`.`), footer arrows (`↑/↓`→`^/v`), and warning signs (`⚠`→`!`) when `reduced_motion` is enabled.
- **Help audit**: Help covers all primary actions: `?`, `s`, `w`, `n`, `r`, `l`, `d`/`t`, task arrows, `f`/`g`, `Esc`, headline arrows/Enter, `q`. View-specific footers cover context-relevant keys.
- **Smoke test**: Binary starts and exits cleanly in a macOS PTY. `start.sh` builds and runs. `config.json` is created beside the executable.

### Deferred

- Item 6 (ticket-11 reference captures) pending ticket 11 visual overhaul completion.
