# Zenko Browser

A fast everyday browser in Zenko's night-shrine style that talks to Zenko.

- **Engine:** WebView2 (the Edge/Chromium engine already on Windows 10/11), so sites, sign-ins, video and PDFs work like in Edge.
- **Shell:** Rust (`tao` + `wry`), a few MB, starts instantly. The frame (tabs, toolbar, command bar) is a web page drawn by
  Chromium's GPU compositor, so animations run at your screen's full refresh rate.
- **Built by GitHub Actions** (`.github/workflows/browser.yml`): nothing to install on your PC.

## 0.4.0 (this version)

- **Subject tab groups.** Tabs about the same subject gather after a coloured label in Zenko's subject colours (Bio green, Chem blue, Maths amber, French rose, Psych violet, English red, Core gold). Two tabs of one subject make a group by themselves; right-click a tab > Subject group to choose one or "No group". Click a label to fold the group, right-click it to fold, open a tab in it, send its pages to Zenko notes, ungroup or close it. Drag a tab onto a label or into a group to move it. Ctrl+Tab follows the order you see. Settings > Shape and layout > Group tabs by subject.
- **A little fox lives in the tab bar.** Very discreet: every few minutes a tiny half see-through fox peeks up from the bottom of the empty part of the tab bar, strolls across it, sits and swishes her tail, or curls up for a nap. She only uses empty space and slips away when the mouse comes near. Settings > Motion and sparkle (Off / Discreet).
- **Night shrine theme.** A very dark, low-contrast colour scheme with warm lantern light and fireflies drifting in the sky instead of foxfire. Settings > Colours, or "Night shrine after 10 pm" to switch to it automatically at night.
- **Focus mode follows Zenko.** Starting a focus session or lock-in in Zenko turns focus mode on here at once, and off again when the session ends (unless you turned it on yourself). Settings > Focus mode > Turn on with Zenko study sessions.
- **The shoji screen.** A blocked site now fades away as two shoji screens slide shut, lit from behind, with a hanging ema showing your Zenko session and the minutes left.

## 0.3.0
A real link with Zenko. A fox button in the toolbar glows when Zenko is open (both apps leave each other a small "I'm here"
file in `%APPDATA%\FoxBuddy`, nothing goes online) and shows what's coming up. Right-click now has a **Zenko** submenu:
summarize the selection or the whole page (Zenko's summary pops up in her bubble and chat; without Zenko a quick summary is
made on your PC), make a flashcard, quiz me on this page (fill-in-the-blank cards), add a selected date to Zenko as an
Upcoming deadline or an ema reminder, send text, links or the page to her notes. More right-click items: search the web for
the selection, copy as plain text, count words, copy page link, open image in a new tab. The tab right-click menu keeps
everything it had and adds Copy link, Copy clean link and the Zenko submenu. The new tab shows **Zenko's calendar**: the
next 14 days with dots for deadlines, the upcoming list and her plan for today. No school calendar is read: only what you
add to Zenko yourself.

## 0.2.0
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
