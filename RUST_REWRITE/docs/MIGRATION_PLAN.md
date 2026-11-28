# Zenko: Python to Rust migration plan

Source of truth: `zenko.py` at the repo root (zenko23, `APP_VERSION = "v2026.11.25-zenko23"`). This work never edits it.
Everything Rust lives in `RUST_REWRITE/`. The full per-function list is in [AUDIT_INVENTORY.md](AUDIT_INVENTORY.md)
(regenerate it with `python RUST_REWRITE/tools/audit_inventory.py`).

## 1. What zenko.py is (Phase 1 audit)

- 61,308 lines. About 45,500 are code; the rest are two embedded data blobs: `_LEX_BLOB` (Strong's Hebrew + Greek lexicon
  for the Bible window, xz + base85) and `_FONT_BLOB` (Zen Maru Gothic + Zen Old Mincho fonts).
- 43 classes, 591 module functions, 1,798 functions and methods in total.
- One process, one Tk root. The desktop fox is an always-on-top, borderless Tk window whose magenta (`#ff00ff`) pixels are
  see-through. The hub (Home, Timetable, Chat, Settings and every other page) is a second Tk window.
- Each release adds a subclass on top and overrides methods with `super()`:
  `FoxBuddy(ZenkoGym, ZenkoBooks, ZenkoStudy, ZenkoLife, ZenkoExtras)`, then `Zenko` (zenko15-18), `Zenko19`, `Zenko20`,
  `Zenko21`, `Zenko22`, `Zenko23`. `main()` runs `Zenko23`. Some releases also replace module functions in place
  (zenko22 replaces `_motif` and adds 7 travel robes to `ROBES`; zenko21 wraps `sprite_photo` with HD smoothing; zenko20 wraps
  the sprite builders with a disk art cache). The port follows the *final* behaviour, which is why every Rust frame is checked
  against the running Python module, not just against the source text. (The first test run caught exactly this: the
  zenko22 travel-robe patterns.)
- Data lives in `%APPDATA%\FoxBuddy\` (config.json, stats.json, timetable.json, learned.json, zenko_extras.json,
  zenko_life.json, notes, sounds, fonts, art cache, logs). The Rust app must use the same files in the same format so you can
  switch between the two without losing anything.

### The character rendering pipeline (Phase 2)

1. Every picture is a 110 x 78 grid of palette letters (`.` = clear). Shapes are cell sets from ellipses, polygons, capsules
   and cubic curves, shaded with a light from the top left and a 4x4 Bayer ordered dither, outlined with `K`/`k`.
2. Layers: tails (`build_tails`, a separate layer so she never bobs), body + ema plaque (`_body`), head (`_head`), face
   (`eyes`, `blush`, `mouth`), accessories (`glasses`, `headphones`, `draw_outfit`), robes (`robe_sit`), effects.
   Full-body poses use the zenko19 rig (`build_rig`: two-bone IK legs, torso on landmarks, Catmull-Rom outline, fluffy tufts).
   Others: `build_walk4`, `build_walk34`, `build_curl` (asleep), `foxfire_rows`, `z19_sit_extra` (book, dumbbells, zen).
3. Palette: `FUR[theme]` + coat, eye colour, foxfire colour, bib colour (`make_pal`) + robe colours (Zenko19).
4. Raster: letters to RGBA, then zenko21 HD smoothing (Scale2x/Scale3x chain, 5x/7x special case) at size `Z`.
5. Desktop frame (`animate`, every 60 ms): tails, fox, ema clock text drawn as 2Z x 2Z blocks, foxfire wisps and motes.

Exactness notes for the port: Python float maths is reproduced in the same order; `round()` is half-to-even;
`%` and `//` follow Python's sign rules; `math.hypot` uses CPython's own algorithm (not the C library's); dict insertion
order is kept where it changes the result (tail outlines, eye cells).

## 2. Python to Rust dependency map

| Python | Used for | Rust |
|---|---|---|
| tkinter window + `-transparentcolor` | desktop fox | Win32 layered window (`UpdateLayeredWindow`, per-pixel alpha) via `windows-sys` (done) |
| tkinter Canvas/Label/Entry/Text | hub pages, tool windows, bubble | a drawing layer that mirrors the Tk canvas calls (`tiny-skia` shapes, `cosmic-text`/`fontdue` text, Win32 windows), so page code ports line by line and looks the same |
| embedded fonts + `AddFontResourceEx` | Zen Maru Gothic / Zen Old Mincho | the same fonts built into the exe, no install |
| ctypes user32/kernel32/shcore | foreground window, idle time, DPI, hotkeys, media keys, battery | `windows-sys` |
| winsound + MCI | sound effects, background music | `PlaySoundW` + `mciSendStringW` (the same Windows APIs) |
| synthesised WAVs (`_koto`, `_bell`, ...) | all sounds and music | port the maths 1:1 and compare the bytes with Python |
| urllib | weather, search, wiki, news, Gemini, ntfy Town square, updater | Windows' own WinHTTP (net.rs), so school proxies and certificates behave as they do now |
| json | every data file | `serde_json` (same files, same keys) |
| threading + queue | brain, web, voice, prewarm | `std::thread` + channels |
| re, difflib, unicodedata | chat routing, typo fixes | `regex`, `strsim`, `unicode-normalization` |
| html.parser | page text for search summaries | `scraper` / `html5ever` |
| ast (safe calculator), cmath, fractions | chat maths, Casio CG50 window | a small expression parser, `num-complex`, `num-rational` |
| lzma + base85 blobs | lexicon, fonts | built into the exe as plain bytes |
| zipfile, hashlib | updater, installer, art cache | `zip`, `sha2` |
| subprocess (Ollama, VOICEVOX, Python workers for faster-whisper / piper) | local brain, voice | `std::process`; Ollama and VOICEVOX are HTTP and port directly; the whisper/piper workers stay external tools at first |
| SAPI through COM | the old Windows voice | `windows` crate COM |
| sounddevice + numpy (microphone) | offline voice chat | `cpal` or `waveIn` |
| PyInstaller exe + Releases self-update | install and updates | a plain `ZenkoRust.exe` (no Python, no PowerShell) + the same Releases updater |

## 3. Feature inventory and status

**done** = ported and checked against Python; **partial** = part of it; blank = not started yet.

| Area | Features | Status |
|---|---|---|
| Sprite engine (module, ~1,700 lines) | tails, sitting body + ema, head, 3 ear styles, 3 blush styles, eyes/mouth for all 26 states, effects, glasses, headphones, outfits (flower, bow, crown, headband, mask, towel, sweat), bib, all 13 robes and patterns, walk4, walk34, rig + 15 poses + walk, curl, foxfire, palettes (10 coats, 8 themes, eye/foxfire/bib colours), HD smoothing at every size | **done** (653 frames + 127 images identical) |
| Desktop fox | window size and place, idle sway, ear flick, paw lick, blink, look at mouse, tails grow with focus minutes, seasonal robe, ema clock (24h/12h) and session countdown, foxfire wisps and motes, sleep/curl when away, drag | **partial** (ZenkoRust.exe preview) |
| Sounds | every synthesised sound and voice clip (koto, suzu, furin, taiko, her 20 voice lines, the zenko14 sound pool, "a+b" mixes), zenko17 voice-only beep, varied play sounds, VOICEVOX clip pick-up, ui sounds | **done** (279 .wav files byte-identical to Python's) |
| Play reactions | click regions (nose, head, belly, tails), pet/belly/boop/tickle/tail/treat/yarn/hide-and-seek/high five/gift/dance/brush/sing, props, sulking after 8 pokes, forgiving, bond, treats, treasures, hunger | **done** (logic tested; poses use the checked sprite engine) |
| Speech bubble | zenko22 say/spot/place: size scaling, bubble styles, Zen Maru Gothic text (same fonts built in), wrap, chips, frame, knot, tail, left/right/above/below placement, follows her, tap to dismiss | **done, not yet seen on Windows** (outline shapes match Python on 420 cases; drawn with the same GDI calls Tk uses) |
| Right-click menu | every item of on_menu + both menu_extras, same order and words, theme colours; size, colours, mood, sound and other toggles write config.json | **partial** (items whose feature isn't ported yet are greyed out) |
| Focus sessions | start, pause/resume, stop, countdown on the ema, idle pause, end lines, stats.json, the new-session dialog, timer panel, distraction detection (foreground window, browser titles, site rules, study videos), nags, praise, pep talks, auto-close, breaks, learned habits | **done, not yet seen on Windows** (distraction decisions match Python on 25 titles; dialog layout matches real Tk) |
| Roaming and poses | walks along the taskbar, chases the mouse, hops home, zenko19 four-legged walk, stretch / play bow / pounce / rest / sniff routines | **done, not yet seen on Windows** |
| Lock-in | timer, hachimaki, Esc-hold exit, pounce-and-eat of distractions (tab close / minimise / close), the zenko21 overlay animation, eaten counts, resume after restart | **done, not yet seen on Windows** |
| Desktop fox, rest | music tablet, battery, weather particles, the visiting fox, slide, focus glow, exam countdown on the ema | **music tablet, weather particles + real weather, visiting fox, 30 s life tick done**; battery, slide, glow, ema countdown to do |
| Session rewards | garden growth, journey minutes, badges, hunger after a session or lock-in, timetable marking, pomodoro, music hooks | **all done** (byte-for-byte vs zenko.py), except the Spotify-account music hooks |
| Hub shell | sidebar with pixel icons, rearrange, accents, text scaling, themed scrollbars, Ctrl+K search, page transitions, living sidebar fox | **done** (window frame, sidebar, living fox, page slide, Ctrl+K palette, focus glow, themed scrollbars, text re-fit) |
| Home, Daily, You | streak, mood/hunger, 61 badges, check-ins, journal | **done (4a)**: pages match zenko.py, golden-tested |
| Timetable, Upcoming, Japanese clock, planner, reminders rack | free hours, half-hour slots, plan/replan, countdowns, emas that auto-plan, PDF export | **done (4b)**: pages match zenko.py, golden-tested |
| Chat + brain | commands, shorthand learning, her voice, Ollama local models, online brain (Gemini) with IB-terms recheck, web search + summaries, Markdown/LaTeX, screenshot reading | **offline router about half done** (core, version routes, Exercise, books); brains/web to do |
| Study | Notebook tutor, Notes scratchpad + folder import, Books + practice, IA/EE/TOK, grades, flashcards/SRS, pop quizzes | **pages done (4d)**; the Notebook tutor's brain is 5b |
| Tool windows | periodic table, French dictionary, Casio CG50 calculator, Bible with Greek/Hebrew lexicon | **done (5a)**; their brain answers are 5b |
| Exercise | 3-day split, pixel muscle map, weight, calories, food lookup, gym look | **done (4e)** except the online food lookup (5b) |
| Garden, zen garden, breathing, koi fishing | seasons, day/night, wilting, 32 fish, collection book | **done (4e)**: scene, koi and tones bit-exact; page, cards and koi book match zenko.py |
| World map, journey, Town square | shrines to the Silk Road, quizzes, rewards, real-date festivals, ntfy chat street | **done (4f)**: paintings bit-exact, pages match zenko.py, the link's parsing golden-tested |
| Screen time | per-app/site time, private windows skipped | **done (4e)** |
| Voice | VOICEVOX auto-start + speech, chirps, offline voice chat, Piper, SAPI | **written** (5c), untested on Windows |
| Sounds and music | synthesised UI sounds, 3 background tracks | **done**: 279 sounds and the 3 loops byte-identical |
| Installer and updater | self-install, Releases updater, crash logs | **written** (5d; preview installs as ZenkoRust), untested on Windows |
| Browser link | Linked light, browser_inbox.json, focus with Zenko | **done** (5d) |

## 4. How the port is checked (Phase 6)

- `tools/make_golden_sounds.py` records the sha256 of every .wav zenko.py synthesises (all kinds, volumes 1-3, two voice
  pitches); `tools/make_golden_ui.py` records round_pts outlines. `cargo test` checks both.
- `tools/make_golden.py` imports the untouched zenko.py and dumps what it draws (grids, palettes, final HD pixels) into
  `golden/`. `cargo test` redraws every case in Rust and fails on a single different cell or pixel.
- Re-run `make_golden.py` whenever zenko.py changes; if its drawings changed, the Rust tests fail until the port catches up.
  (A CI job that does this automatically on GitHub is ready to add when wanted.)
- The same method carries on for later stages: sound WAV bytes, PDF output, planner results, chat routing, and screenshots
  of pages drawn by both apps.

## 5. Order of work (small, testable stages)

1. **Done:** sprite engine, palettes, HD raster, golden tests.
2. **In progress:** desktop fox window. Done: sounds, play reactions, speech bubble, right-click menu, sessions with the
   dialog and timer panel, distraction checks, nags, pep talks, breaks, roaming and poses, lock-in with the pounce.
   Stage 2c done: session and lock-in rewards (rewards.rs).
   Stage 2d done: pomodoro, timetable alerts/marking, background music, music tablet, weather, visiting fox, life tick.
   Stage 3a done: battery on the tail, ema countdown, daily check-in card. Vertical slide and focus glow belong to
   the hub window, so they move to step 4.
3. **Started (stage 3a):** a Tk-compatible drawing layer (ui.rs + uiwin.rs): Windows Tk widget sizes, pack, place,
   cards, pixel buttons, entries, checked against real Tk geometry and canvas items. Still to add as pages need them:
   grid in the tree, images, aa_photo, smooth lines (Text widgets with tags and ZkScrollbar are in).
4. **Started (stage 3b):** the hub shell: canvas painting with Tk's Windows rules (canvasdraw.rs + tkpaint.rs), the
   window frame (hubwin.rs), the sidebar (hubside.rs) and its life (hubanim.rs), the Windows window (hubw.rs).
   Stage 3c done: page switching with the vertical slide, the Tk text widget, Markdown/LaTeX, the Ask Zenko page and
   the offline chat router's core (help, sessions, breaks, check-in, breathing, teach/forget, shorthand, poses, tips,
   jokes, lore), all checked against zenko.py.
   Stage 3d done: ema reminders, the study planner with emas that plan themselves, and the version routes (z23, z22,
   z18, z17, z16, z15, ib, garden, Exercise, study books), all checked against zenko.py.
   Stage 3e done: zenko8's study tracker (window ticks, subject asks, exam credit, recap, weak-subject alert, golden
   hour, missed-session re-planning) and study_route, checked against zenko.py on 112 steps.
   Stage 3f done: the rest of the offline chat (calculator, extras, life chat, flashcards/quiz, screen, Spotify,
   apps, deadlines, battery, notes), checked against zenko.py on 229 lines plus 200+ calculator cases.
   Stage 3g done: deadline reminders, the sleep/burnout/water guard, screen-time recording, the minute checks (pop
   quiz, IB and garden nudges) and the focus glow, checked against zenko.py.
   Stages 4a-4e done: the hub pages Home, Daily, You, Timetable, Upcoming, Reminders, Settings, Customize (with Ctrl+K),
   Notebook/study Books, Notes, Screen time, Exercise and the Zen garden (breathing, grounding, koi fishing, the koi
   book window), each matching zenko.py's widget tree, canvas items and pictures in recorded cases.
   Stage 4f done: the World map and the Town square (festival street, chat board, ntfy link).
   Stage 5a done: the periodic table, French dictionary, CG50 calculator and Bible windows.
   Stage 5c done: VOICEVOX, voice chat, Piper and the Spotify account.
   Stage 5d done: installer, updater, crash logs and the Zenko Browser link.
   **Next:** 6, final checks; then 5b brains/web (deferred by the user to the end).
5. Brain, web, voice, music, tool windows, games, Town square, updater/installer.
6. Speed (Phase 5): image cache and native code are already in; measure and tune at the end.

Size: the drawing engine is about 4% of zenko.py's code (96 of 1,798 functions). The rest is roughly 43,000 lines of Python,
so the full port is many sessions of work, not one. Each stage ends with something you can run and compare.
