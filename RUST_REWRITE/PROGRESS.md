# Rust rewrite progress

Branch `claude/project-thread-cxqzjv` (PR #18). Full plan: docs/MIGRATION_PLAN.md. Rules: RULES.md.
243 of 1,798 zenko.py items ported (`python3 tools/audit_inventory.py`).

## Done
- **Stage 1:** crates/zenko-art is an exact port of the sprite engine.
- **Stage 2a:** crates/zenko-sound, play reactions, speech bubble, right-click menu, basic sessions.
- **Stage 2b (latest, commits 471e7b1 and the lock-in commit after it):**
  - distraction watch, nags, pep talks, auto-close and breaks (watch.rs)
  - timer panel (panel.rs)
  - new-session dialog (dialog.rs, tkw.rs)
  - Tk grid/pack layout maths (tkgrid.rs)
  - window icon (icon.rs)
  - roaming and the zenko19 routines (life.rs)
  - lock-in (lockin.rs) and the zenko21 pounce overlay (eat.rs)

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
| Behaviour, roaming, lock-in | Rule tests |

## Open problems
- Nothing has run on real Windows yet (Wine is broken here). The bubble, menu, panel, dialog and pounce still need screenshots from the user.
- Sessions and lock-ins don't give their rewards yet: garden growth, journey minutes, badges, hunger.

## Next recommended task
The rewards from sessions and lock-ins: garden growth, journey minutes, badges, hunger, timetable marking and pomodoro. After that: the music tablet, weather and the visiting fox.

## Gotchas
- The repo root has a tracked `__pycache__/`. Never delete it, and run Python with bytecode writing turned off.
- Cross-compile with `cargo build --release --target x86_64-pc-windows-gnu`.
