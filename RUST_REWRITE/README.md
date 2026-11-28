# Zenko in Rust (work in progress)

A line-by-line port of `zenko.py` to native Rust. The Python app is the reference: nothing here changes it.

- `crates/zenko-art`: Zenko's pixel-art renderer (every sprite, palette and the HD smoothing), checked cell for cell against zenko.py.
- `crates/zenko-sound`: every sound and voice clip zenko.py synthesises, byte for byte the same .wav files.
- `crates/zenko-desktop`: `ZenkoRust.exe`, the desktop fox: window, speech bubble, right-click menu, play, sessions.
- `assets/fonts`: Zen Maru Gothic + Zen Old Mincho (SIL OFL), copied out of zenko.py by `tools/extract_fonts.py`.
- `golden/`: frames dumped from zenko.py by `tools/make_golden.py`.
- `docs/MIGRATION_PLAN.md`: the audit, the Python-to-Rust map, status and plan. `docs/AUDIT_INVENTORY.md`: every function.

Build and test:

```
python RUST_REWRITE/tools/make_golden.py          # (repo root) refresh the reference frames from zenko.py
python RUST_REWRITE/tools/make_golden_sounds.py   # ... and the reference sounds
python RUST_REWRITE/tools/make_golden_ui.py       # ... and the UI box shapes
cd RUST_REWRITE
cargo test --release                              # every Rust frame, sound and shape must equal the Python one
cargo build --release -p zenko-desktop            # on Windows: target/release/ZenkoRust.exe
cargo run --release -p zenko-desktop --example preview -- out.gif 6 focus   # any OS: animated GIF of the desktop fox
```

ZenkoRust.exe uses the same files as zenko.py in `%APPDATA%\FoxBuddy` (config.json, stats.json, zenko_extras.json,
zenko_life.json, sounds\). It only changes the keys it uses, the way zenko.py writes them. Don't run both at once.
