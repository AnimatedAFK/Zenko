# Zenko in Rust (work in progress)

A line-by-line port of `zenko.py` to native Rust. The Python app is the reference: nothing here changes it.

- `crates/zenko-art`: Zenko's pixel-art renderer (every sprite, palette and the HD smoothing), checked cell for cell against zenko.py.
- `crates/zenko-desktop`: `ZenkoRust.exe`, the desktop fox window (preview).
- `golden/`: frames dumped from zenko.py by `tools/make_golden.py`.
- `docs/MIGRATION_PLAN.md`: the audit, the Python-to-Rust map, status and plan. `docs/AUDIT_INVENTORY.md`: every function.

Build and test:

```
python RUST_REWRITE/tools/make_golden.py          # (repo root) refresh the reference frames from zenko.py
cd RUST_REWRITE
cargo test --release -p zenko-art                 # every Rust frame must equal the Python one
cargo build --release -p zenko-desktop            # on Windows: target/release/ZenkoRust.exe
cargo run --release -p zenko-desktop --example preview -- out.gif 6 focus   # any OS: animated GIF of the desktop fox
```

The preview exe only reads your Zenko data in `%APPDATA%\FoxBuddy` and keeps its own position in `rust_preview.json`.
