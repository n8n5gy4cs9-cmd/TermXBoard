# 02 — Portable preferences

**What to build:** First-run and Settings flows that keep city, seven named Themes, and reduced-motion preference beside the distributed executable without scattering application data elsewhere.

**Blocked by:** 01 — Launchable TermXBoard shell.

**Status:** done

- [x] First run proposes Turku, Finland and persists the accepted or entered city.
- [x] Settings selects any canonical Theme and toggles reduced motion; both survive restart.
- [x] Builds preserve existing preferences; unwritable storage falls back to session-only settings with a clear warning.
- [x] Preference defaults, persistence, invalid data, and fallback behavior have automated tests.
