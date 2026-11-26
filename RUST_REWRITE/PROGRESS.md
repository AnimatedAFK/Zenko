# Rust rewrite progress

Branch `claude/project-thread-cxqzjv` (PR #18). Full plan: docs/MIGRATION_PLAN.md. Rules: RULES.md.
278 of 1,798 zenko.py items ported (`python3 tools/audit_inventory.py`).

## Done
- **Stage 1:** crates/zenko-art is an exact port of the sprite engine.
- **Stage 2a:** crates/zenko-sound, play reactions, speech bubble, right-click menu, basic sessions.
- **Stage 2b (commits 471e7b1 and the lock-in commit after it):**
  - distraction watch, nags, pep talks, auto-close and breaks (watch.rs)
  - timer panel (panel.rs)
  - new-session dialog (dialog.rs, tkw.rs)
  - Tk grid/pack layout maths (tkgrid.rs)
  - window icon (icon.rs)
  - roaming and the zenko19 routines (life.rs)
  - lock-in (lockin.rs) and the zenko21 pounce overlay (eat.rs)
- **Stage 2c (latest):** what a finished session or lock-in earns (rewards.rs): hunger, the 61 ema badges (plain,
  tiered, secret: Friday 13th, full moon, New Year, birthday, 3 am, dawn), garden xp/health/stage-ups, journey minutes
  and new places, koi bait/gold, the zenko22 counters and 3-minute badge tick, unlocked accent colours (theme.rs).
  zenko_life.json gets zenko.py's start-up setdefaults in its order, so a file written by Rust is identical.
  Files: rewards.rs (new), data/rewards.json (tools/gen_rewards_data.py), lib.rs (`set_fake_now`, helpers),
  session.rs, lockin.rs, play.rs, theme.rs, main.rs (`life_startup`, `pump` each frame).

## Architecture decisions (settled)
- **Logic lives in the library.** zenko-desktop lib = platform-free logic, tested on Linux. src/main.rs and the Windows-only modules (gdi, bubble, menu, panel, dialog, tkw, eat) handle the Win32 side.
- **The library never calls the OS.** Companion emits `Ev` events (Say, Beep, Ui, Command). The window side feeds it `Env` (work area, idle time, dragging) and reads `win_move`.
- **Drawing copies Tk's own GDI calls.** Shapes and widget geometry are ported from Tk's C source (tcltk/tk core-8-6-branch).
- **Data files are written exactly like Python's.** Same files in %APPDATA%\FoxBuddy and the same JSON format, using read-modify-write of only our own keys.
- **Quirks are copied too.** One example: zenko.py's window-edge nap never fires, so the port leaves it out.

## Tests (all passing on 2026-11-26)
Run with `cargo test --release`. The golden files are made by the tools/ scripts.

| Area | Checked against |
|---|---|
| Sprite art | zenko.py's drawings |
| Sounds | zenko.py's .wav files |
| UI shapes | zenko.py's outlines |
| Distraction decisions (25 window titles) | zenko.py |
| Icon (8 themes) | zenko.py |
| Dialog layout | Real Tk, via `xvfb-run -a /usr/bin/python3.12 tools/make_golden_dialog.py` |
| Session and lock-in rewards (14 cases) | zenko.py's own methods, byte-for-byte zenko_life.json/stats.json, via `tools/make_golden_rewards.py` |
| Behaviour, roaming, lock-in | Rule tests |

## Open problems
- Nothing has run on real Windows yet (Wine is broken here). The bubble, menu, panel, dialog and pounce still need screenshots from the user.
- Chips like "Show me" / "See the map" send `hub:garden` / `hub:map`, which do nothing until the hub exists.
- Not ported yet from end_session: timetable marking, pomodoro, music hooks; z18_init's one-time move of old chat
  reminders onto the ema rack.
- The rewards test doesn't replay the 5 s follow-up badge check (in these cases it awards nothing in Python either).

## Next recommended task
Pomodoro and timetable marking at session end, then the music tablet, weather and the visiting fox. After that, a
Tk-like drawing layer for the hub pages (Garden, Map, Daily), which the reward chips open.

## Gotchas
- The repo root has a tracked `__pycache__/`. Never delete it, and run Python with bytecode writing turned off.
- Rewards golden: `python3 tools/make_golden_rewards.py` pins the clock (UTC) and stubs only screen parts.
- Cross-compile with `cargo build --release --target x86_64-pc-windows-gnu`.
