# Zenko changes

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
