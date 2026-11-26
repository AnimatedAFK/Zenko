//! zenko-art: Zenko's procedural pixel-art renderer, ported exactly from the original zenko.py.
//!
//! The Python original stays the source of truth; `tests/golden.rs` compares every frame this crate draws
//! with frames dumped from zenko.py by `RUST_REWRITE/tools/make_golden.py`.

pub mod core;
pub mod curl;
pub mod palette;
mod palette_data;
pub mod raster;
pub mod rig;
pub mod sit;
pub mod walk;

pub use crate::core::{Grid, H, W};
pub use crate::sit::Look;
