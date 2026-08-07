# 09 — Live project refresh

**What to build:** Continuous Task View updates from the active Progress File with visible but unobtrusive change and health feedback.

**Blocked by:** 07 — Status-first Task View.

**Status:** done

- [x] The active Progress File is re-read every 60 seconds while Task View is active; `r` refreshes immediately.
- [x] Valid changes update the board, briefly highlight changed Tasks, and show an update timestamp.
- [x] Missing, partial, or malformed updates retain the last valid Project and show a red error LED/message.
- [x] Refresh scheduling, change detection, view exit, and recovery have automated tests.
