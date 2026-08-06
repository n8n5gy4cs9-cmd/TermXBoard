# 10 — Integrated polish and verification

**What to build:** A coherent finished TermXBoard experience that unifies all views, controls, visual states, Themes, fallbacks, and release verification.

**Blocked by:** 03 — Live computer telemetry; 04 — Live weather; 05 — Developer news Dashboard; 08 — Task filtering and grouping; 09 — Live project refresh.

**Status:** ready-for-agent

- [ ] All seven Themes remain readable across Dashboard, Task View, menus, errors, details, and resize state.
- [ ] Decorative animation respects reduced motion; true-color and icon failures degrade to readable ANSI/text output.
- [ ] Help documents every key and refresh action; navigation works without a mouse.
- [ ] Automated tests pass and a concise manual smoke test covers live terminal rendering, networking, scripts, and the distributed executable.
