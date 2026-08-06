# 03 — Live computer telemetry

**What to build:** Dashboard cards showing macOS CPU, memory, battery, disk, network, and uptime with compact sci-fi visuals and honest availability states.

**Blocked by:** 01 — Launchable TermXBoard shell.

**Status:** ready-for-agent

- [ ] Available telemetry refreshes every 2–5 seconds without blocking input or clock updates.
- [ ] Unavailable metrics show an explicit unavailable state rather than fabricated values.
- [ ] Green, orange, and red LEDs consistently communicate healthy, loading, and error states.
- [ ] Metric formatting, refresh state, and unavailable behavior have automated tests.
