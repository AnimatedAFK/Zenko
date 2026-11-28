# Rust rewrite progress

Branch `claude/project-thread-cxqzjv` (PR #18). Full plan: docs/MIGRATION_PLAN.md. Rules: RULES.md.
402 of 1,798 zenko.py items ported by `python3 tools/audit_inventory.py` (its list was last updated at 3g; stages 4a-4e aren't in it yet).

## Stages left to completion (6, as of 2026-11-28; 4f done)
1. ~~3e: zenko8 study tracking + study_route~~ (done)
2. ~~3f: the remaining chat routes~~ (done)
3. ~~3g: desktop leftovers~~ (done; Ctrl K moved to 4c, since it searches the Settings/Customize cards)
4. ~~4a: hub pages Home, Daily, You~~ (done)
5. ~~4b: Timetable, Upcoming (Japanese clock), Reminders, Deadlines pages~~ (done)
6. ~~4c: Settings and Customize pages, plus the Ctrl K search palette (ZkPalette, z20_items)~~ (done)
7. ~~4d: Study pages: Notebook, study Books, Notes~~ (done)
8. ~~4e: Exercise, Garden (zen garden, fishing, koi book), Screen time pages~~ (done)
9. ~~4f: World map and Town square~~ (done)
10. ~~5a: tool windows: periodic table, French dictionary, CG50 calculator, Bible~~ (done)
11. 5b: brains and web: Ollama, online brain, web search + summaries, food lookup, screenshot reading (DEFERRED by the user on 2026-11-28: do it last, after 6)
12. ~~5c: voice (VOICEVOX, voice chat, Piper) and the Spotify account~~ (done)
13. ~~5d: installer, updater, crash logs, Zenko Browser link~~ (done)
14. 6: final checks. Done here: full test run (86 pass), clean Windows build, docs/LEFTOVERS.md (tools/leftovers.py). Still needs the user: running ZenkoRust.exe on real Windows and the side-by-side look check

## Done
- **Stage 5d:** installer, updater, crash logs and the Zenko Browser link.
  - update.rs (lib): version_tuple/version_newer, write_lnk's [MS-SHLLINK] bytes, update_check (GitHub latest
    release or latest.json), the exe check (MZ, 2 MB), update_download (sha256, 400 MB cap, .part), zk_log_error.
  - install.rs (bin): the preview installs beside the Python Zenko as **ZenkoRust** (own folder in
    LOCALAPPDATA\Programs\ZenkoRust, shortcut, uninstall key and Run value), updates only from a ZenkoRust.exe release
    asset (never zenko.py / Zenko.exe), and shares zenko.py's single-instance mutex so the two never run together on
    the same data folder. --uninstall, --portable, --updated (25 s wait), panic hook to error_log.txt, the 20 h
    update check with Update now / Skip bubble, Settings' Check now / From a file / Install buttons.
  - zblink.rs (lib): browser_inbox.json every 5 s (notes, picture clips, reminders, deadlines, summaries, cards,
    fill-in-the-blank quizzes), zenko_link.json every 15 s and at once when a session/lock-in starts or stops (with
    session and ema_plan, as Zenko23), the reading note, the sidebar's link light, and the built-in summariser
    (summarise_offline/summarise_extract/_sentences/_terms/_stem, also needed by 5b's web summaries).
    Bin: notes into the open Notes page's scratchpad, "Save to notes", "Quiz me" (hub_ask).
  - Decisions: the first-run installer is a Yes/No/Cancel message box instead of zenko.py's Tk installer window
    (same choices; Cancel sets install_skip). Summaries use the built-in summariser until 5b brings the online brain.
  - Tests: update.rs (3), zblink.rs (2). Golden: make_golden_update.py, make_golden_zblink.py.
- **Stage 5c:** voice and the Spotify account.
  - voice.rs (lib): VOICEVOX finders, clip names, speaker list, STT status text; unzip.rs; spotify.rs (PKCE login on
    127.0.0.1:8765, token refresh, api with 401 retry / NO_DEVICE / 403 / 429, now playing, presets, play_query).
  - Bin: voicehost.rs (engine auto-start/stop, speech with bubble timing like zenko.py's, beeps hushed while she talks,
    Customize demo/find/browse/menu), ears.rs (waveIn mic with VAD, whisper-cli or Windows speech, hotkey hold, talk
    mode, whisper/piper installers), speak.rs Piper, spotifyhost.rs (connect/forget, now-playing poll and song
    announcement, tablet and Home buttons, presets).
  - Tests: voice.rs (5), spotify.rs (3); rewards.rs now accepts SayAloud. Golden: make_golden_voice.py.
- **Stage 5a (85e4705 .. this one):** the four study-tool windows.
  - cgmath.rs: the CG50 maths (cg_eval grammar, exact fractions with limit_denominator, %g, roots, linear systems,
    the solver, stats, regression, normal/binomial distributions). ptable.rs: the elements and molar_mass.
    bible.rs: book data, html_unescape, step_parse_line, bundled lexicons (data/lexicon.json), morph_text.
  - toolwin.rs (ZkToolWindow look and geometry), ptwin.rs, frwin.rs, casio.rs (every key mode, keyboard, blink,
    life cg with Python's list aliasing), biblewin.rs (header nav, paned page with verse tags, flowing word cards,
    lexicon entry, explain/note helpers).
  - web.rs: http_get/http_json/_download/quote/translate_text with zenko.py's WebError texts and a SHA-256; the
    request goes through a fetcher the bin registers (WinHTTP, now with Content-Length) so the lib stays testable.
    biblefs.rs: bible_chapter (bolls, bible-api, KJV fallback, disk cache), STEPBible index/sections split into
    chapter files, the format check, step_have_all/download_all, bible_original, the lexicon.
  - Bin: toolwins.rs hosts all four (one each; open_ptable / open_french_dict / open_calc / open_bible[:ref] from
    chat, Books and Ctrl K), threads for the translator and Bible jobs, the 530 ms calculator blink, life pt_last /
    fr_hist / cg / bible_pos / bible_notes, the Bible's book and translation menus, note and study windows.
    uiwin.rs gained down/motion/leave/char events, minsize, hwnd_of and origin.
  - Tests: tool_maths.rs (4), tool_windows.rs (pt 2 cases, fr 3, cg 55 key steps, bible 2: tree, text lines),
    biblefs.rs (network + disk with a mock fetcher). Golden: make_golden_tools.py, make_golden_page.py tools.
  - Deferred to 5b: the brain answers (Ask about the element, Explain the grammar / Examples, Explain this verse);
    they show zenko.py's "no brain" message for now. Bible study notes already cached in life still show.
- **Stage 4f (commits 905508c .. this one):** the World map and the Town square.
  - mapart.rs / mappage.rs / hubmap.rs: the 13 region paintings bit-exact, the page (region bar, painting with pins,
    torii, road, clouds, ring and mini Zenko, hover tip, passport stamps, journey rows, the gold-seal quiz) in 3 cases.
    A right quiz answer adds the seal to life journey.quiz, plays the gift sound, says "Sugoi~!" and checks badges.
  - squareart.rs: Z23 festival street paintings (24 streets), crowd looks and sprites (60 people), the festival
    calendar, square_clean / square_name_ok / square_tag. squarepage.rs: Z23Street (crowd, named people, speech bubbles
    that float above each other, lanterns, fireworks, ambient, bats, Holi puffs) and the page (status row, board,
    entry, rules note), checked in 3 cases (offline winter night, live Christmas Eve dusk with 5 messages in a small
    window, Holi morning). squarelink.rs: SquareLink23's take/visible/post (16 stream lines, reports, looks, posts).
  - Bin: net.rs `request()` (WinHTTP, http/https, any method, body, streamed reading); sqnet.rs (the ntfy stream thread
    with zenko.py's backoff, posts on threads, connect 15 s after start when square_on, look updates); hubsquare.rs
    (800 ms paint, 90 ms street, Go online / rules bubble, Send with slow mode and the put-back on failure, report /
    block / copy menu on a message, the street's yukata / mask / festival preview menu, stall clicks -> z23_stall in
    squarelink.rs). Settings: the square_on check connects / disconnects, the server box is validated like setsrv.
  - Quirks copied: hub_build_square paints before the page is mapped, so the board starts at the top; the board's
    line numbers ignore the offline/quiet note's two lines (as zenko.py's ui["lines"] does).
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

- **Stage 4a:** the Home, Daily and About you pages, each matching zenko.py's widget tree (place, size, text,
  font) in every golden case, plus their canvases (week chart, mood dots, 61-badge shelf) and pictures.
  homepage.rs, dailypage.rs, youpage.rs (new), pagekit.rs (new: hub_card, tt_btn/tt_entry, hub_scroller + wheel +
  scrollbar, glow_bar, zk_fonts, Zenko21's zk_wrap with z21_unwrap / flow + grid reflow / column stacking /
  z21_note_reflows). ui.rs: Checkbutton/Radiobutton (Windows and X11 geometry, `ui.x11` for tests), editable Text,
  label -width, pack(in_=), lift, uniform grid columns, destroy. hubw.rs: the three pages built on first show, Home's
  count-up and 1 s refresh, its buttons/Return/music keys, You's Save (button, Return, leaving a field), switches and
  Forget everything, Daily's moods, cards, check-in, breathing (command), homework, facts add/forget, badge views,
  badge hover tips and pin clicks, daily_refresh every second (skipped while typing), wheel on all three.
  Golden: tools/make_golden_page.py home|you|daily (records wrap pieces for wrapped labels and canvas text; the
  resized "small" cases also record them near the built font sizes). Tests: tests/home_page.rs, you_page.rs,
  daily_page.rs. The resized cases replay zenko.py's order (fonts scaled at the first size, first wrap at the new size).
  The breathing window itself isn't ported yet (garden, 4e); its button sends open_breathing.
- **Stage 3g:** guard.rs: deadline_reminders (15 s after start, then every 30 min), books_slow every 30 s
  (wind-down, bedtime nag with "10 more min" / "Goodnight~", burnout cap with a 15-min break, the rest-day and
  late-night lines; bedtime/wake default to DEFAULT_CONFIG's 23:00/06:00) + gym_water_nudge, screen_tick after
  study_tick (screen_classify/screen_site/screen_excluded, private windows remembered by handle, YouTube study
  videos, idle limits, the 1-hour screen_tell; counts wait in a cache that read_json applies, like zenko.py's
  in-memory life) and zk_slow_tick every minute (idle_quiz_check's pop quiz + Show answer / I knew it / Forgot /
  Later, ib_nudge_check, garden_check, screen_save_maybe, journey_check). Env.bubble = the speech bubble is up.
  hubwin.rs: z19_glow (the progress line and its head in the frame, every 120 ms) and the petal burst after a
  completed session. main.rs: "hub:<page>" chips now open pages, "lockin:N" works, quiz_on() sees a running quiz.
  zenko.py quirk copied: quiz_offers is cleared before its count is read back, so it is always 1.
- **Stage 3f:** the rest of the offline chat. calc.rs: Python's safe calculator (same regex prep, a
  Python-grammar parser, exact big ints, CPython float/complex pow, math errors, repr with "," grouping).
  extra.rs: extra_route (lock-in, pomodoro, focus N, reminders -> emas, weekly report, quiz, calc, define/translate
  web jobs, notes search with difflib ratio, todos, omikuji with weighted picks, JP word of the day, sing, outfits,
  bib, coats, rename, treats, plays, treasures, bond, sulk/sorry). lifechat.rs: life_route's rest (homework ->
  notebook, exams in words, free time, plan, "need N hours", countdown, flashcards + SRS review, quiz answers,
  facts, music, toggles, walk, streak/badges, how are you, haiku, visitor). miscroutes.rs: screen, Spotify (typo
  pass), apps (open_app_hook, Windows side to come in 5a), deadlines (find_dates/find_time), battery, notes LLM.
  Fixed on the way: ema relative times use Python datetime microseconds (py_dt_add), the planner announces from the
  time the line was sent (Planner.asked_at), "review my mistakes", the gift line goes through a timer, pick/haiku/
  visitor use the seeded rng like Python.
- **Stage 3e:** study.rs, zenko8's study smarts. study_tick (1 s, after tick_1s records the window) with
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

- **Stage 4b (commits 713fbad..fd057f7):** Timetable, Upcoming (clock painting, menus, calendar, IA/EE/TOK tracker) and
  Reminders (night shrine, plaques, wisps) pages, each matching zenko.py's golden cases and wired into the hub.
- **Stage 4c (601b1a9..8f8592b):** Settings and Customize pages; the Ctrl+K palette (search, items, reveal + glow),
  Ctrl+1..9, background page prebuild.
- **Stage 4d (ba03a4c..feb6422):** Study books shelf, the 8 open book pages (layout and pixels), practice generators and
  answer checks, the Books page in the hub, the Notes page (scratchpad filing, notebooks, pages).
- **Stage 4e (aa0cb92..b8c60bf, latest):** Screen time page; Exercise page; the Zen garden:
  - gardenart.rs: the garden scene (raster, tree, seasons, sky), bit-exact (golden/garden.json, 12 cases).
  - fishart.rs: all 32 koi (fish_pattern with CPython str seeding, fish_rgba), bit-exact (golden/fish.json, 192 cases).
    mt.rs gained random.gauss and str seeds. zenko-sound breath_tone: the breathing cue .wavs byte-exact (breath.json).
  - gardenpage.rs: the page (painting with Zenko by her tree, lanterns, fireflies, petals, pond koi, decor, the
    breathing guide over a dimmed garden, side panel with Breathe / Koi fishing / Grounding / ema tree / Recent / How
    it grows), the cast/bobber/ripples/jump frame, and the fishing cards (question, escape, catch with its buttons).
    Golden: make_golden_page.py garden, 6 cases (frames held still, one frame at a fixed clock and random 0.99).
  - koibook.rs + koiwin.rs: the koi book as its own titled window (tile grid with silhouettes, scroll, detail panel,
    pond toggle, hints). Golden: make_golden_page.py koibook, 3 cases. uiwin.rs gained open_window (a framed,
    resizable window that sends clicks/wheel/keys/resize to the app) and paints Tk canvases in popups too.
  - hubgarden.rs: the 50 ms garden_anim, foxfire flicker, breath_start/stop/finish (calm log, +0.1 health, done line,
    "Well done~", the tones via PlaySound), 5-4-3-2-1 grounding, Lock in / Plant a new seed, fishing (bait, free casts,
    golden bait, fish_question from your cards or the bank, fish_roll, fish_answer with srs_update, saved misses,
    pond/book/log, all fishing badges, z22_badge_check), the subject menu, keys 1-4 / Esc / Return.
    main.rs: chat and menu commands breath:k, grounding, fish_cast, koi_book open the garden and run there.

## Architecture decisions (settled)
- **Logic lives in the library.** zenko-desktop lib = platform-free logic, tested on Linux. src/main.rs and the Windows-only modules (gdi, bubble, menu, panel, dialog, tkw, eat) handle the Win32 side.
- **The library never calls the OS.** Companion emits `Ev` events (Say, Beep, Ui, Command). The window side feeds it `Env` (work area, idle time, dragging) and reads `win_move`.
- **Drawing copies Tk's own GDI calls.** Shapes and widget geometry are ported from Tk's C source (tcltk/tk core-8-6-branch).
- **Data files are written exactly like Python's.** Same files in %APPDATA%\FoxBuddy and the same JSON format, using read-modify-write of only our own keys.
- **Quirks are copied too.** One example: zenko.py's window-edge nap never fires, so the port leaves it out.

## Tests (all passing on 2026-11-28: 86 tests)
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
| Calculator: 200+ expressions (ints, floats, complex, errors, grouping) | zenko.py's calc_eval, `tools/make_golden_calc.py` |
| Extra/life/misc chat: 229 lines (replies, bubbles, commands, extras, cfg, life, badges, emas, timetable, deadlines, session, pomodoro, lock-in, quiz, sulk) | Zenko23.chat_send with seeded random, `tools/make_golden_extra.py` |
| Sleep/burnout/water guard, deadline note, screen time (16 window kinds), pop quiz + taps, IB and garden nudges: 64 steps | zenko.py's own methods, `tools/make_golden_guard.py` |
| Focus glow + petal burst: every frame item in 10 cases | zenko.py's z19_glow on a recording canvas, `tools/make_golden_glow.py` |
| Zen garden scene (12), koi art (192), breathing tones (12) | zenko.py's pixels / .wav bytes, `tools/make_golden_garden.py`, `make_golden_fish.py`, `make_golden_breath.py` |
| Zen garden page: 6 cases (whole tree, every canvas item and picture, question + catch cards) and the koi book window: 3 cases | Real zenko.py under Xvfb, `tools/make_golden_page.py garden` / `koibook` |
| World map: 13 paintings (pixels) + the page in 3 cases (tree, every canvas item) | Real zenko.py, `tools/make_golden_map.py`, `tools/make_golden_page.py map` |
| Town square: 24 streets, 60 people, calendar, filters; the page in 3 cases; the link on 16 stream lines + 3 posts | zenko.py, `tools/make_golden_square.py`, `make_golden_page.py square`, `make_golden_squarelink.py` |
| Tool windows: periodic table (2), French dictionary (3), CG50 (55 key steps), Bible (2); CG maths, molar mass, Bible parsing, network/disk layer | Real zenko.py, `tools/make_golden_tools.py`, `make_golden_page.py tools` |
| Updater: shortcut bytes, version compare, update_check answers, exe check, download; browser link: summariser, cloze/split cards, inbox end to end, link file | zenko.py, `tools/make_golden_update.py`, `make_golden_zblink.py` |
| VOICEVOX finders/speakers, STT status, Spotify login/refresh/api with a pretend server | zenko.py, `tools/make_golden_voice.py` |
| Version routes, Exercise, study books: 127 lines (replies, bubbles, what opens, IB tasks, rules, gym, timetable, grades, session, bedtime) | zenko.py with screen time/garden/stats/browser link set up, `tools/make_golden_routes.py` |

## Open problems
- Nothing has run on real Windows yet (Wine is broken here). The bubble, menu, panel, dialog and pounce still need screenshots from the user.
- Town square: the street rebuilds at once on a resize (zenko.py waits 250 ms); the street's random stream and the
  stalls' picks are Rust's own (zenko.py's are unseeded). The live ntfy link is untested here (no network to ntfy.sh
  in this sandbox): its parsing is golden-tested, the WinHTTP stream needs a try on Windows.
- Garden: fishing and breathing use Rust's random stream (zenko.py's is not reproducible either); breath_start doesn't hush
  VOICEVOX yet (5c). The garden cards' long labels aren't re-wrapped by zk_wrap (zenko.py's 140 ms after-map pass).
- The sidebar's side scenes are off (z22_unlocked isn't ported). Only the smooth motion
  level (45 ms) is used. The tkpaint StrokeInPieces path follows Tk core-8-6-branch; older 8.6 builds may differ.
- Voice, Spotify, the installer and updater are written but untested on Windows: the mic (waveIn), whisper/Piper
  helpers, VOICEVOX's run.exe, the Spotify browser login, registry/shortcut writes and the self-replacing update all
  need a real try. z18_init's one-time move of old chat reminders onto the ema rack isn't ported.
- Zenko Browser link: the session key the browser sees is a hash of the session (zenko.py uses id()); the same
  task restarted within 2 s keeps the key. A summary's "Reading..." bubble shows for 0.3 s (the built-in summariser
  is instant here; zenko.py runs it on a thread).
- All offline chat routes are ported except web_route/start_web (5b). Lines that need a brain or the web are sent as
  commands nobody handles yet: define:, translate:, quiz_start:, spotify:, ask_screen:, summarise_file:, nb_show:*,
  nb_start_test (4d/5b/5c). last_file is never set (attachments, 5b). Homework passes
  only subject_in (nb_sid isn't tracked until the Notebook page, 4d). plan_short {} comes from the Upcoming page's
  up_plan (4b); the extra test normalises it.
- Not ported: the food lookup online (an unknown
  food answers like a failed lookup), the Exercise tab's own early tick that fills today's food list.
  The smart brains (Ollama, online), web lookups, Mic/Talk/Image/Screen buttons and attachments are not ported.
- The rewards test doesn't replay the 5 s follow-up badge check (in these cases it awards nothing in Python either).

## Next recommended task
5b (brains and web), then the non-5b leftovers in docs/LEFTOVERS.md (reMarkable link, timetable PDF export, dropped/attached files). Before that, the user should try ZenkoRust.exe on Windows: every Win32 part (bubble, menus, mic, VOICEVOX, Spotify login, installer, updater) is untested there.

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
- make_golden_extra.py moves the fake clock 2.7 s and sleeps 2.7 s per line, so Tk's real after() timers and the
  fake clock stay in step; the Rust test pumps at now+2.7. Python's idle pose lines (IDLE_LINES) are filtered out.
- make_golden_guard.py cancels the start-up greeting's bubble_job (or the pop quiz never shows) and reseeds random
  inside screen_tell (other after() jobs can use random first); the Rust test reseeds before pump the same way.
- make_golden_page.py garden drops after(50) frames, the 230 ms foxfire flicker and breath_start's second panel, pins
  time.monotonic/random.random only inside `_final`, and ends with zk_fit (a panel rebuilt there misses its Map scaling).
- Cross-compile with `cargo build --release --target x86_64-pc-windows-gnu`.
