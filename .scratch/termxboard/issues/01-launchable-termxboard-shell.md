# 01 — Launchable TermXBoard shell

**What to build:** A runnable Rust terminal application that opens Dashboard with clock/date, keyboard help and quit controls, a safe resize screen, and portable start/build commands that produce a distributable TermXBoard executable.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] Dashboard starts in a modern macOS terminal; `?` shows help and `q` exits immediately.
- [ ] Terminals below 110×32 show the resize screen and recover automatically.
- [ ] Start and build commands fail clearly without prerequisites; build produces the executable under `dist/`.
- [ ] Core launch, input, and size-state behavior has automated tests.
