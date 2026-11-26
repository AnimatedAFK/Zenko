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
| urllib | weather, search, wiki, news, Gemini, ntfy Town square, updater | `ureq` with Windows' own TLS (schannel), so school proxies and certificates behave as they do now |
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
| Desktop fox, rest | speech bubble window, timer panel, music tablet, battery, props, play reactions (pet, boop, belly, tails, sulk, treats), life poses (roam, walk across the screen, slide), lock-in pounce, focus glow, exam countdown on the ema, the full right-click menu, sounds | |
| Focus sessions | focus dialog, sessions, breaks, distraction detection (foreground window and browser titles), nags, praise, pep talks, auto-close, lock-in, stats.json | |
| Hub shell | sidebar with pixel icons, rearrange, accents, text scaling, themed scrollbars, Ctrl+K search, page transitions, living sidebar fox | |
| Home, Daily, You | streak, mood/hunger, 61 badges, check-ins, journal | |
| Timetable, Upcoming, Japanese clock, planner, reminders rack | free hours, half-hour slots, plan/replan, countdowns, emas that auto-plan, PDF export | |
| Chat + brain | commands, shorthand learning, her voice, Ollama local models, online brain (Gemini) with IB-terms recheck, web search + summaries, Markdown/LaTeX, screenshot reading | |
| Study | Notebook tutor, Notes scratchpad + folder import, Books + practice, IA/EE/TOK, grades, flashcards/SRS, pop quizzes | |
| Tool windows | periodic table, French dictionary, Casio CG50 calculator, Bible with Greek/Hebrew lexicon | |
| Exercise | 3-day split, pixel muscle map, weight, calories, food lookup, gym look | |
| Garden, zen garden, breathing, koi fishing | seasons, day/night, wilting, 32 fish, collection book | |
| World map, journey, Town square | shrines to the Silk Road, quizzes, rewards, real-date festivals, ntfy chat street | |
| Screen time | per-app/site time, private windows skipped | |
| Voice | VOICEVOX auto-start + speech, chirps, offline voice chat, Piper, SAPI | |
| Sounds and music | synthesised UI sounds, 3 background tracks | |
| Installer and updater | self-install, Releases updater, crash logs | |
| Browser link | Linked light, browser_inbox.json, focus with Zenko | |

## 4. How the port is checked (Phase 6)

- `tools/make_golden.py` imports the untouched zenko.py and dumps what it draws (grids, palettes, final HD pixels) into
  `golden/`. `cargo test` redraws every case in Rust and fails on a single different cell or pixel.
- Re-run `make_golden.py` whenever zenko.py changes; if its drawings changed, the Rust tests fail until the port catches up.
  (A CI job that does this automatically on GitHub is ready to add when wanted.)
- The same method carries on for later stages: sound WAV bytes, PDF output, planner results, chat routing, and screenshots
  of pages drawn by both apps.

## 5. Order of work (small, testable stages)

1. **Done:** sprite engine, palettes, HD raster, golden tests.
2. **Started:** desktop fox window. Next: speech bubble, full right-click menu, focus sessions and timer panel, play
   reactions, roaming/walking, lock-in, sounds. At the end of this stage ZenkoRust.exe can replace Zenko on the desktop.
3. A Tk-compatible drawing layer and the fonts, checked by screenshot comparison with Tk.
4. Hub pages one at a time (Home, Timetable/Upcoming, Chat, Settings/Customize first, then the rest), each compared with Python.
5. Brain, web, voice, music, tool windows, games, Town square, updater/installer.
6. Speed (Phase 5): image cache and native code are already in; measure and tune at the end.

Size: the drawing engine is about 4% of zenko.py's code (96 of 1,798 functions). The rest is roughly 43,000 lines of Python,
so the full port is many sessions of work, not one. Each stage ends with something you can run and compare.
