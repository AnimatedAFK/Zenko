# Zenko changes

## v2026.11.24-zenko20

Zenko 20 is the speed and ease update. Everything she could do before still works, and your saved data and settings are untouched.

- **Much faster.** The window now opens about 0.6 s after start instead of 1.6 s, and switching between pages is near instant (all 15 pages: 4.9 s before, 0.7 s now, measured on the same PC).
  - Every picture Zenko paints (her frames, tails, walk, the sidebar fox, the shelf, the muscle map, the wave banner) is remembered in `%APPDATA%\FoxBuddy\cache`, so it is painted only once, ever, instead of on every start.
  - Her next frames are pre-drawn in the background, so walking and poses never stutter the first time.
  - Pages get ready quietly in the background while you aren't touching anything, so clicking one shows it straight away (Settings > Speed and smoothness can turn this off).
  - When Zenko.exe runs a newer zenko.py from an update, it now compiles it once and reuses that (0.57 s down to 0.06 s per start).
- **Calmer on your PC.** Animations slow down when the window is hidden or you're in another app, and nothing is redrawn unless it changed. New **Motion** choice in Customize > Look and feel: Auto (calmer on battery), Smooth, Balanced or Battery saver.
- **Quick search (Ctrl+K)**, or click the search bar at the top: type a few letters to jump to any page, any settings card (it scrolls there and glows), a study book ("bio", "chem"), a theme or accent, or an action like box breathing, the periodic table, the calculator, a focus session or a break. Anything else you type can go straight to Chat.
- **Shortcuts**: Ctrl+1 to Ctrl+9 open the sidebar pages in order, Ctrl+, opens Settings, Alt+Left goes back to the page you were on.
- **Arrange the sidebar** (Customize > Sidebar pages): tick which pages show and move them up or down. Hidden pages are still one Ctrl+K away. There's also a compact sidebar option.
- **Accent colours** (Customize > Look and feel): Gold, Torii, Sakura, Jade, Sky, Wisteria or Moonlight on top of any fox theme, or keep the theme's own.
- Small polish: the Home button glow, tips and the chat "thinking" dots no longer repaint every frame, and the search bar fits its text.

## v2026.11.24-zenko19.1

- Zenko is fluffy again when she walks and does her poses. She has a round, soft body with a fluffy coat edge, a big white chest ruff under her chin, round haunches, thick fluffy legs instead of thin dark "stockings", round little paws like the ones she shows when sitting, and bigger cloud-like tails.
- The real fox skeleton and movement are unchanged: the 4-beat walk with a little hop, the stretches, the play bow, lying down, and the mousing pounce.
- The outfits still fit over the new fluffy body.

## v2026.11.23-zenko19
- **A brand-new walk**: she now walks on all four paws like a real fox, built from a proper skeleton (shoulder, elbow, wrist and paw in front; hip, knee, hock and paw behind), using the real four-beat fox/dog walk (left hind, left front, right hind, right front). The black "stockings" on her legs, small paws and a long brush of tails are kept, and there's a little hop in her step.
- **Real fox poses**, all on the same skeleton: front stretch (forearms flat, rump up, big yawn), back-leg stretch, play bow with a swishing tail, lying down like a sphinx and dozing off, sniffing the ground, a shake-off, and the mousing pounce (crouch, rear up on the hind legs, leap, dive front paws first, pinned!). She does them by herself now and then when you're not focusing, or ask in chat: "stretch", "play bow", "pounce", "lie down", "sniff". Customize has buttons for each and an on/off switch.
- **Wardrobe** (earned with study hours): seasonal kimono (sakura in spring, ro silk in summer, momiji in autumn, snow in winter), a festival yukata, and a sailor uniform for exam season. "Dress for the season" switches by itself: uniform when an exam is within 3 weeks, yukata in late summer, otherwise the season's kimono. Chat: "wear the yukata", "dress for the season".
- **The sky follows the clock, gently**: the sidebar and Zen garden take on a soft dawn, a deep jade-blue day and a plum-and-ember dusk; night is the same shrine night as before. The moon rests while a hazy sun is up.
- **Seasons and weather**: sakura petals, fireflies, falling momiji or snow in the sidebar; a sakura or maple branch in the corner; snow on the torii and hills in winter; petals, leaves or a dusting of snow in the Zen garden; rain when it's raining where you live (needs your city in Settings).
- **Pixel-art menu icons** in her gold style (torii, scroll, hourglass, ema, book, dumbbell, bonsai, compass...).
- **The little fox at the top acts out the page**: reads a book in Study books, lifts dumbbells in Exercise, meditates in the Zen garden, trots on the World map, lies down in Settings, talks in Chat.
- **Page transitions**: a thin foxfire wisp sweeps across with the slide when you switch pages.
- **Focus glow**: during a focus session a dim foxfire line fills the window's wooden frame as a progress bar (only in the frame, so it never covers anything), and petals run round the frame when you finish.
- **Settings navigation bar**: a quiet row (Look, Focus, Voice, Life, Brain, Apps, Data) under the title jumps to each section and follows you as you scroll.
- **Study alerts use her voice**: "Heads up: ... in 5 min" and reminders are now said in her VOICEVOX voice (in Japanese, the bubble stays English) instead of the Windows voice. If VOICEVOX isn't running she does her little "kon kon~" clip. Settings > Sky, seasons and motion lets you pick the Windows voice again or silence.
- Every new effect has its own switch in Settings > Sky, seasons and motion.

## v2026.11.23-zenko18.1
- **Bible word lookups work with no internet**: the full Strong's Hebrew and Greek lexicon (STEPBible, CC BY 4.0, about 23,000 entries) is now built into Zenko, so clicking any word shows its complete dictionary entry even on school wifi.
- If GitHub is blocked, the original-language text now also tries a second download source (jsDelivr).

## v2026.11.23-zenko18
- **Reminders tab (絵馬)**: a night shrine. Each reminder is a wooden ema plaque on a shrine rack, and your planned study for today hangs above as paper lanterns. Write one at the top ("call mum at 6pm", "take vitamins every day at 8") or tap a time. Click a plaque to mark it done (it gets a red 済 stamp and moves to the fulfilled row), edit it, snooze it or take it down.
- **Zenko tells a reminder from a plan**: "remind me to study at 9" hangs an ema; "plan bio at 9am for 45 min" or "I'll study chem at 8pm" goes on your timetable. In chat you can also say "my reminders", "move call mum to 7pm", "mark stretch as done", "rename 2 to ...", "snooze vitamins for 30 min", "delete reminder 3".
- **Both are said out loud** in English when it's time (the Mute button doesn't silence these; the switch on the Reminders page does).
- Pages no longer cut off a column on smaller windows: a column that doesn't fit moves under the other one, and goes back side by side when the window is wide again.

## v2026.11.23-zenko17.2
- **Mute in Chat only mutes her spoken lines** (the Japanese VOICEVOX sentences and read-out replies). Her little voice sounds when you pet or boop her keep playing.
- **She always walks back to the far right** of the screen after wandering, chasing the mouse or napping on a window.

## v2026.11.23-zenko17.1
- **Bible originals work for every verse**: fixed the "No such file" error. The app mixed up "Isa" (Isaiah) with "1 Sa" (1 Samuel), so it sometimes downloaded the wrong part of the Bible. All 1,189 chapters were checked against the STEPBible data. A new "Save all Hebrew + Greek for offline" button saves the whole Old and New Testament once (do it at home if school wifi blocks GitHub). Old, mixed-up saves are cleared once by themselves.
- **Calculator Equation mode** now works like the real one: F1 SIMUL (2 to 6 unknowns), F2 POLY (degree 2 to 6, including complex roots), F3 SOLVER (type any equation, e.g. x^2=2^x or sin(x)=0.5, and it finds every real solution from -1000 to 1000). SHIFT then . types "=".
- **Her voice and bubble start together**: the bubble waits until her voice is ready (at most 5 seconds), and a long line plays as one clip with no gaps. Voice clips are saved, so lines she has said before play instantly.
- **Mute button in Chat**: stops her talking right away (for long answers). Press it again to unmute. It is remembered.

## v2026.11.23-zenko17
- **Her voice matches what she says**: with VOICEVOX on, every speech bubble is said out loud in Japanese (translated for free online, or by your local Ollama model, and remembered so it works offline next time). The bubble stays in English. Talk mode with the anime voice works the same way.
- **Only voice lines**: no more drums, bells, plucks or chimes when you play with her. The interface click sounds are switched off once (Settings > General turns them back on).
- **Close or keep rules** (Settings): simple sentences like "On YouTube, if the title has bio, cell → Keep it open" or "Reddit → Close it". Your rules come first. Also in chat: "keep youtube if it has bio", "always close reddit", "my rules".
- **Study tool windows** (each opens in its own window): a periodic table with IB data-booklet values, electron configurations and a molar-mass box in the Chemistry book; a French dictionary and translator in the French book; and an fx-CG50-style graphing calculator (Run, Graph with roots/max/min/intersections, Equation, Statistics, Normal/Binomial distributions) in the Math book.
- **Bible book** on the shelf (opens in its own window): NKJV fetched chapter by chapter (it is copyrighted, so it can't be built in) with KJV/WEB/ASV/YLT; click any verse for the original Hebrew, Aramaic or Greek word by word (STEPBible data, CC BY 4.0) with Strong's numbers, grammar in plain English and lexicon entries; "Explain this verse" writes study notes; Bible Hub interlinear and commentary links; your own notes per verse.
- Themed scrollbars, and the tutor and sources answer boxes scroll with the mouse wheel.
- Voice input fixes: newer whisper.cpp builds print nothing with -np, so the words are read from a text file; a muted microphone is reported; a blocked PowerShell speech helper now offers the offline engine instead of failing silently.
- Steadier maximise/fullscreen: memory clean-up only runs on the window thread, and a crash log (fault_log.txt) is kept if it ever happens again.
- Many more haiku, and she doesn't repeat one until about 40 others have been said.

## v2026.11.22-zenko16.1
- **Study videos really get through now**: YouTube videos about your subjects (like "Cellular Respiration", "How do vaccines work?", "SN1 vs SN2 reactions") are no longer eaten in lock-in or flagged in focus sessions. Two bugs: any title with the word "reaction" was treated as a reaction video, and one-word topics like "Enzymes" were judged too strictly. She now uses a much bigger list of IB topic words for all six subjects and understands plurals.
- YouTube's home and search pages get 75 seconds so you can find your video before she pounces.
- Settings > Study helpers: add your own words that mean a video is for study (a teacher's or channel's name).

## v2026.11.22-zenko16
- **Zen garden**: the Garden tab is now a calm, moonlit karesansui (raked gravel, mossy rocks, stone lantern, water basin, temple wall) with no big red blocks. Your tree still grows from focus.
- **Breathing exercises**: box breathing (before an exam), 4-7-8 (before sleep), slow breathing and the calming sigh, with a guided glowing circle, counts, and soft synthesised sounds. Plus a 5-4-3-2-1 grounding card.
- **Talk out loud**: press Talk (chat window or right-click menu) and have a spoken conversation. Speech-to-text runs offline with whisper.cpp (one-time ~60 MB download in Settings > Talk with Zenko). She answers in a clear English voice or her VOICEVOX anime voice.
- **Self-updating from zenko.py**: Zenko.exe now checks this repo's releases and runs a newer zenko.py (SHA-256 checked) without a rebuild; if the new code fails to start it falls back to the built-in version.
- Fix: lock-ins and focus sessions now really grow the garden tree again.
