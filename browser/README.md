# Zenko Browser

A fast everyday browser in Zenko's night-shrine style that talks to Zenko.

- **Engine:** WebView2 (the Edge/Chromium engine already on Windows 10/11), so sites, sign-ins, video and PDFs work like in Edge.
- **Shell:** Rust (`tao` + `wry`), a few MB, starts instantly. The frame (tabs, toolbar, command bar) is a web page drawn by
  Chromium's GPU compositor, so animations run at your screen's full refresh rate.
- **Built by GitHub Actions** (`.github/workflows/browser.yml`): nothing to install on your PC.

## 0.2.0 (this version)
Everything from stage 1, plus: Edge import (sign-ins, passwords, bookmarks, history), address-bar autofill, ad/tracker blocker
with a per-site shield, cookie pop-up rejecting, link cleaning, YouTube ad + sponsor skipping, focus mode (block list, strict
allow-list, 5-minute pass), spaces, subject tab groups, tab tree, split view, peek, profiles (two accounts side by side),
auto-archive, unread badges, search inside open tabs, side panels (notes per page, downloads, history timeline, media, today's
recap), downloads sorted into subject folders, save page offline, citations (MLA/APA), translate, word lookup, clip to Zenko,
reminders to Zenko, InThinking key terms to Zenko, video tools + mini player, link hints, mouse gestures, per-site dark mode /
font / zoom / hide, draft keeping, scroll restore, crash recovery, no-autoplay, battery saver, Outlook chime, start-up routines,
new-tab widgets (timetable, tasks, weather, time online, notes), default-browser registration, unpacked Chrome extensions.

## Stage 1
Tabs on top or on the side (drag to reorder, middle-click to close), session restore, Ctrl+K command bar (tabs, sites, commands,
inline maths), Claude side panel (resizable, pop-out window), shrine new-tab page, one heavily customisable theme (8 Zenko palettes or
your own accent, fonts, tab shapes, roundness, spacing, size, stars, glow, motion), remappable shortcuts, saved passwords and autofill,
tab sleep, mute per tab, video full screen, DevTools, print/PDF, auto-update from GitHub Releases, and the Zenko link
(window title + `%APPDATA%\FoxBuddy\browser_now.json`, so Zenko logs study time per subject).

## Releases
Publish a **pre-release** with a tag like `browser-v0.1.0` (must match `version` in `Cargo.toml`). The workflow attaches
`ZenkoBrowser.exe` + `.sha256`. Pre-release keeps it out of Zenko's own "latest release" updater.

## Data
`%LOCALAPPDATA%\ZenkoBrowser` (session, sign-ins, saved passwords).
