# Zenko changes

## v2026.11.22-zenko16
- **Zen garden**: the Garden tab is now a calm, moonlit karesansui (raked gravel, mossy rocks, stone lantern, water basin, temple wall) with no big red blocks. Your tree still grows from focus.
- **Breathing exercises**: box breathing (before an exam), 4-7-8 (before sleep), slow breathing and the calming sigh, with a guided glowing circle, counts, and soft synthesised sounds. Plus a 5-4-3-2-1 grounding card.
- **Talk out loud**: press Talk (chat window or right-click menu) and have a spoken conversation. Speech-to-text runs offline with whisper.cpp (one-time ~60 MB download in Settings > Talk with Zenko). She answers in a clear English voice or her VOICEVOX anime voice.
- **Self-updating from zenko.py**: Zenko.exe now checks this repo's releases and runs a newer zenko.py (SHA-256 checked) without a rebuild; if the new code fails to start it falls back to the built-in version.
- Fix: lock-ins and focus sessions now really grow the garden tree again.
