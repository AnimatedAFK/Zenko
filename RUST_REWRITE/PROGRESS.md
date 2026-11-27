# Rust rewrite progress

Branch `claude/project-thread-cxqzjv` (PR #18). Full plan: docs/MIGRATION_PLAN.md. Rules: RULES.md.
402 of 1,798 zenko.py items ported (`python3 tools/audit_inventory.py`).

## Stages left to completion (13, as of 2026-11-27; 3e done)
1. ~~3e: zenko8 study tracking + study_route~~ (done)
2. 3f: the remaining chat routes (life_route rest, extra_route incl. pomodoro/notes/todos/calc, apps, deadlines,
   battery, quiz + flashcard review)
3. 3g: desktop leftovers: focus glow, Ctrl K, deadline reminders, screen-time recording, books_slow (bedtime guard,
   6 h burnout line, water nudge)
4. 4a: hub pages Home, Daily, You
5. 4b: Timetable, Upcoming (Japanese clock), Reminders, Deadlines pages
6. 4c: Settings and Customize pages
7. 4d: Study pages: Notebook, study Books, Notes
8. 4e: Exercise, Garden (zen garden, fishing, koi book), Screen time pages
9. 4f: World map and Town square
10. 5a: tool windows: periodic table, French dictionary, CG50 calculator, Bible
11. 5b: brains and web: Ollama, online brain, web search + summaries, food lookup, screenshot reading
12. 5c: voice (VOICEVOX, voice chat, Piper) and the Spotify account
13. 5d: installer, updater, crash logs, Zenko Browser link
14. 6: testing on real Windows, speed, final side-by-side check

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
- **Stage 2c:** what a finished session or lock-in earns (rewards.rs): hunger, the 61 ema badges (plain,
  tiered, secret: Friday 13th, full moon, New Year, birthday, 3 am, dawn), garden xp/health/stage-ups, journey minutes
  and new places, koi bait/gold, the zenko22 counters and 3-minute badge tick, unlocked accent colours (theme.rs).
  zenko_life.json gets zenko.py's start-up setdefaults in its order, so a file written by Rust is identical.
  Files: rewards.rs (new), data/rewards.json (tools/gen_rewards_data.py), lib.rs (`set_fake_now`, helpers),
  session.rs, lockin.rs, play.rs, theme.rs, main.rs (`life_startup`, `pump` each frame).
- **Stage 2d (latest):** tt.rs (timetable load/save, 5-minute and "time for" alerts, the "Start timetable item" menu
  line, ticking an item when its session completes, studied/counted + Ready to Fly badge, the zenko23 "take its ema
  down?" offer, pomodoro rounds), speak.rs (SAPI voice for remind_voice = english), zenko-sound/music.rs (the 3
  background loops, byte-identical float32 maths, CPython 3.12 Neumaier sum), bgm.rs (MCI player, spotify_window,
  media keys), music.rs (music_apply, player_poll, the tablet picture and buttons), slow.rs (30 s life tick, haiku,
  found gifts, facts, visiting fox, seasons/weather particles, weather URLs), net.rs (WinHTTP + weather_poll).
- **Stage 3a (latest):** battery meter on her tail (battery.rs), ema exam countdown on the belly plaque, and the
  Tk-like widget layer every window from here on is built with: ui.rs (lib: widget tree, Windows Tk 8.6 sizes for
  labels/entries/frames/canvases/PixelButton, tkPack.c pack, tkPlace.c place, round_frame cards, canvas items, a
  display list, pointer/keys/entry editing; fonts through the `Fonts` trait) and uiwin.rs (bin: GDI fonts, display
  list to gdi::Op, borderless topmost popups with the KEY colour see-through). First user: the daily check-in card
  (checkin.rs: open_checkin, checkin_save, chat_line, should_speak; opened by the 30 s tick, the menu and later chat).
  Fixed: the session dialog's entries were 2 px too small (tkEntry.c inset = highlight + border + XPAD).

- **Stage 3b:** the hub window. canvasdraw.rs (lib: a Tk canvas painted the way Tk 8.6 on Windows does:
  DisplayRectOval / polygon / line / text / image procs, TkMakeBezierCurve smoothing, ComputeTextBbox and image anchors,
  BlendComplexAlpha), tkpaint.rs (bin: tkWinDraw.c's pens, paths, StrokeInPieces, arcs, baseline text, photo blends),
  ui.rs canvases holding a tk canvas (`Draw::Tk`), place bordermode ignore, lowered widgets. hubwin.rs (the frame:
  zk_border wood tiles + fittings + crest, hub_titlebar with the Ctrl K pill and window buttons, tip bar with the
  fading TIPS, stage, zk_factor/zk_size label fonts, hub_zone, nav_list, badge pins, status line), hubanim.rs (stars
  twinkle, seasonal particles, the living logo with blink/head sway/curl and PAGE_POSE poses), hubw.rs (bin: the
  borderless window, DWM round corners, fade-in, drag, resize edges, min/max/close, sidebar hover + clicks, 170 ms
  re-fit). Menu "Open Zenko window" and open_hub_on_start open it; pages inside it are still empty.
  Fixed on the way: accessories() lacked headphones-with-music and the gym look.

- **Stage 3c (latest):** the Ask Zenko page and her offline chat. tktext.rs (Tk 8.6 text widget: word/char wrap, tags
  with priorities, spacing, margins, offsets, see/PICKPLACE, yview, dlineinfo/bbox, drawing; `yview scroll units/pages`
  and `moveto` incl. Tk keeping the pixel offset on line scrolls), mdtext.rs + mdtex_data.rs (md_segments, LaTeX and
  chemistry formatting, md_plain), chatpage.rs (open_chat(parent=pg): banner, chips, text, status, entry row, Mute,
  ZkScrollbar; _chat_insert and _chat_pump's typed replies), chat.rs + chat_data.rs + pydifflib.rs (chat_send,
  answer_local's own routes, fix_word/fix_typos/fix_first, expand_shorthand, learn_aliases_from, fox_voice,
  learn/forget/learned_reply with difflib's exact matching, learned_summary, life_forget, z19 poses + outfits, z16
  breathing/grounding/voice, life_route check-in + breathing, ask_llm's no-brain fallback), hubw.rs (pages built on
  first show and placed in the stage, z21_show's vertical slide with the motion level, typed replies every 14 ms,
  Enter/Send, chips, wheel 3 lines a notch, scrollbar drag/page, Mute, hello line on an empty chat).
  Fixed: learned.json's setdefault order (replies, sessions, distractions).

- **Stage 3d (latest):** the chat router's version routes and the planner, each checked line for line against zenko.py.
  ema.rs (zenko18 ema rack: ema_parse/when/left, add/list/move/rename/done/snooze/remove, ringing every 5 s, daily
  repeats; timetable add/next/today), plan.rs (plan_targets from deadlines.json, replan, half-hour plan_study_slots,
  plan_maybe from the 30 s tick, zenko23 emas that plan themselves: z23_ema_task, z23_plan_emas, the 0.9 s announce
  with its "Don't plan this one" button, noplan, "X takes N", "plan my emas", Town square line), z22.rs (fishing, koi
  book, passport, badges, square, Zenko Browser link), routes17.rs (z17 periodic table/calculator/dictionary/Bible refs
  + close/keep rules, z15 screen time read side/map/updates/pause, ib_route IA/EE/TOK words/limits/due dates/exam
  countdown, garden + delay-picker lines), gym.rs (weight + trend, food table with food_find/food_estimate/food_sane,
  water, split day, sleep, workout), books.rs (急 delay with undo / I'm back / bubble buttons, grades from text,
  books, bedtime, vision download line, bio facts, English tips). data/routes.json from tools/gen_routes_data.py.
  Start-up now runs z21_migrate_free (whole free hours become half hours), like zenko.py.
  The windows those lines open (periodic table, calculator, dictionary, Bible, koi book, fishing, delay picker, study
  books) are sent as commands the window side doesn't handle yet.

- **Stage 3e (latest):** study.rs, zenko8's study smarts. study_tick (1 s, after tick_1s records the window) with
  study_classify / study_context_subject (override, notebook page, learned windows, classify_subject, sites, planned
  sessions, study apps/hints), study_commit -> slog/dlog/hourlog/unsorted, study_flush every 60 s (credit_minutes ->
  planned session auto_s and the 70% tick-off, or studied[exam]) + study_tidy, the "studying Bio?" ask with one-tap
  subject buttons (study_sort learns the window and its words), exam_for/planned_now/log_study, the reports
  (subject_minutes, best_hours, best_time_text, distraction_report, week_report, recap_text), study_slow every 30 s
  (study_missed with the re-plan note, the 21:00 recap, weak-subject alert, golden hour, "25 min X" buttons),
  conjugate_fr and study_route (I'm studying X / switch to, stop studying, what am I studying, logging hours,
  goals, recap, reports, notebook, test me, flashcards from notes; eli12 gives her "thinking" line and an
  `nb_ask:` command for the brains). Per-second commits wait in a queue that read_json("zenko_life.json") applies
  and the next save writes, like zenko.py's in-memory life saved once a minute.
  Fixed on the way: badges (and tier-ups) now also go into the chat log 2.5 s later (zenko.py's award does
  say + gift beep + chat_line; Rust only said it).

## Architecture decisions (settled)
- **Logic lives in the library.** zenko-desktop lib = platform-free logic, tested on Linux. src/main.rs and the Windows-only modules (gdi, bubble, menu, panel, dialog, tkw, eat) handle the Win32 side.
- **The library never calls the OS.** Companion emits `Ev` events (Say, Beep, Ui, Command). The window side feeds it `Env` (work area, idle time, dragging) and reads `win_move`.
- **Drawing copies Tk's own GDI calls.** Shapes and widget geometry are ported from Tk's C source (tcltk/tk core-8-6-branch).
- **Data files are written exactly like Python's.** Same files in %APPDATA%\FoxBuddy and the same JSON format, using read-modify-write of only our own keys.
- **Quirks are copied too.** One example: zenko.py's window-edge nap never fires, so the port leaves it out.

## Tests (all passing on 2026-11-27)
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
| Timetable alerts, read-aloud, ticking, ema offer (6 more cases in the same golden) | zenko.py, timetable.json byte-for-byte |
| Background music (3 loops + volume copy) | zenko.py's .wav bytes (sha256), `tools/make_golden_music.py` |
| Music tablet picture/buttons, parse_clock, weather URLs, geocode pick | zenko.py, `tools/make_golden_life.py` |
| Check-in card: 13 widgets' sizes/places + 20 canvas items in 3 themes, journal file | Real Tk running zenko.py's open_checkin / checkin_save, `xvfb-run -a /usr/bin/python3.12 tools/make_golden_checkin.py` |
| Hub sidebar: ~2,200 canvas items, moon/sun pixels, icons, text + picture boxes (4 cases) | Real zenko.py under Xvfb, `tools/make_golden_hub.py` |
| Hub frame: every widget's place/size/colours/fonts, border/title/pill items, wood tile pixels (2 cases) | Real zenko.py under Xvfb, `tools/make_golden_hubwin.py` |
| Living logo: mood, clothes, tails + body pixels on 14 pages x 4 times | Real zenko.py's hub_anim, `tools/make_golden_logo.py` |
| Battery meter, ema countdown | zenko.py draw_battery (golden/life.json), rule test |
| Pomodoro, life tick, visitor, weather particles | Rule tests (tests/pomo.rs, tests/life_bits.rs) |
| Behaviour, roaming, lock-in | Rule tests |
| Text widget: every display line, dlineinfo/bbox, yview after see/font changes | Real Tk (golden/hub_pages.json "texts", `tools/make_golden_page.py`) |
| Chat page: whole widget tree in 4 cases, restored + typed replies, 14 wheel/scrollbar steps, thumb items | Real zenko.py's Ask Zenko page, same tool |
| Markdown/LaTeX/chemistry segments (52 cases) | zenko.py's md_segments, `tools/make_golden_md.py` |
| Offline chat: 147 lines (replies incl. fox voice, brain hand-offs, check-in/breathing, session/break state, emas, timetable) + 3 ema ticks | Zenko23.chat_send with seeded random, `tools/make_golden_chat.py` |
| Planner + emas that plan themselves: 23 steps (replies, bubbles + buttons, whole timetable, plan_short/ema_short, deadline ids) | zenko.py with deadlines.json + free time, `tools/make_golden_plan.py` |
| Study tracker + study chat: 112 steps (window ticks, taps, study_slow, chat; lines, bubbles + buttons, commands, slog/dlog/hourlog/unsorted/win_subj/subj_words/goals/studied/said, timetable, self.st, deadline ids) | zenko.py with deadlines, timetable and study history set up, `tools/make_golden_study.py` |
| Version routes, Exercise, study books: 127 lines (replies, bubbles, what opens, IB tasks, rules, gym, timetable, grades, session, bedtime) | zenko.py with screen time/garden/stats/browser link set up, `tools/make_golden_routes.py` |

## Open problems
- Nothing has run on real Windows yet (Wine is broken here). The bubble, menu, panel, dialog and pounce still need screenshots from the user.
- Chips like "Show me" / "See the map" send `hub:garden` / `hub:map`, which do nothing until those pages exist.
- Only the Ask Zenko page exists; the other pages are empty. Ctrl K does nothing, there is no focus glow yet, and the sidebar's
  "Browser" link dot and side scenes are off (zb_linked and z22_unlocked aren't ported). Only the smooth motion
  level (45 ms) is used. The tkpaint StrokeInPieces path follows Tk core-8-6-branch; older 8.6 builds may differ.
- Not ported yet: the Spotify account (tablet buttons use media keys, like zenko.py without an account), her VOICEVOX
  read-aloud (she does her "kon kon~" clip instead, as zenko.py does when VOICEVOX is off), piper voices, plan_maybe (ema auto-planning), z18_init's one-time move
  of old chat reminders onto the ema rack.
- Chat routes still to port, in zenko.py's order: quiz (quiz_answer first, review_start / "review my mistakes"),
  the rest of life_route (homework...), extra_route (pomodoro, notes, todos, calc, fortune...), app_route,
  deadline_route, battery, screen, music, notes, web. Until then such lines can land on a later core route or the
  no-brain reply. study_maybe_ask doesn't see a running quiz yet (quiz_on() is false).
- Not ported: screen_tick (screen time recording; the summary reads what zenko.py recorded), deadline_reminders,
  books_slow (bedtime guard, burnout line, water nudge), the food lookup online (an unknown
  food answers like a failed lookup), the Exercise tab's own early tick that fills today's food list.
  The smart brains (Ollama, online), web lookups, Mic/Talk/Image/Screen buttons and attachments are not ported.
- The rewards test doesn't replay the 5 s follow-up badge check (in these cases it awards nothing in Python either).

## Next recommended task
Stage 3f: life_route's rest (10330), extra_route (7876, pomodoro ->
`pomo_start`), app_route (25359), deadline_route (25097), battery, notes_route (24723), each appended to
tools/make_golden_routes.py's CONVO (seeds go by index), plus the quiz (quiz_start 7700s, quiz_answer 9488,
review_start 9537). Then focus glow (37653), Ctrl K, and the hub
pages one by one (each: a golden dump with tools/make_golden_page.py's walk(), then the Rust tree checked against it).

## Gotchas
- The repo root has a tracked `__pycache__/`. Never delete it, and run Python with bytecode writing turned off.
- Rewards golden: `python3 tools/make_golden_rewards.py` pins the clock (UTC) and stubs only screen parts.
- make_golden_chat.py sleeps 0.25 s per line so after(200) actions land on their own line, and records the session's
  elapsed time before each line ("pre"); the Rust test sets it. Python's chat_log must be kept (the page greets an
  empty log).
- make_golden_plan.py / make_golden_routes.py stub the other timers on the class (study_missed, deadline_reminders,
  check_timetable, update_tick, journey_check, z22_badge_check, _life_slow) and point FOX_FOOD_BASE at a dead port.
  Copy Python lists/dicts (json round trip) before recording them: later lines mutate them in place.
- make_golden_study.py also stubs `tick` (the app's own 1 s tick would overwrite _last_fg with the empty Xvfb
  window), `books_slow_tick` and `study_slow_tick`, pins time.monotonic, and sleeps 2.7 s per step so badges'
  after(2500) land on their own step.
- Cross-compile with `cargo build --release --target x86_64-pc-windows-gnu`.
