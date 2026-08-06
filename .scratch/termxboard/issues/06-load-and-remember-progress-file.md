# 06 — Load and remember Progress File

**What to build:** Safe loading of one Progress File from a launch argument or Dashboard menu, with a remembered-project shortcut and recoverable errors.

**Blocked by:** 02 — Portable preferences.

**Status:** ready-for-agent

- [ ] Absolute and relative paths load through both launch argument and keyboard-driven menu.
- [ ] The last successfully loaded absolute path becomes the Remembered Project without auto-opening next launch.
- [ ] Unknown fields are tolerated; missing required data, missing files, and malformed JSON produce clear errors without losing valid state.
- [ ] No TASKS.md or inferred project data is read, generated, or required.
- [ ] Loading, validation, path handling, and remembered-project behavior have automated tests.
