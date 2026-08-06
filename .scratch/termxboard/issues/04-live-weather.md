# 04 — Live weather

**What to build:** A Dashboard weather card backed by wttr.in for the configured city, showing condition, temperature, feels-like temperature, humidity, wind, and last update time.

**Blocked by:** 02 — Portable preferences.

**Status:** done

- [x] Weather uses Celsius and km/h and refreshes every 15 minutes or through its manual action.
- [x] Loading and failure states use the agreed LEDs and never block Dashboard interaction.
- [x] The last successful session result remains visible during later network failures.
- [x] Response parsing, refresh timing, caching, and error states have automated tests.
