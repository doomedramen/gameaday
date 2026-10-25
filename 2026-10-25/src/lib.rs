//! Slipstream Cup - a top-down arcade racing championship.
//!
//! All simulation, rendering and sound synthesis live in this library; the
//! binary is a thin window/audio adapter.
#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    clippy::manual_is_multiple_of,
    clippy::precedence,
    clippy::chunks_exact_to_as_chunks
)]
pub mod ai;
pub mod app;
pub mod assets;
pub mod audio;
pub mod campaign;
pub mod canvas;
pub mod car;
pub mod font;
pub mod math;
pub mod pngout;
pub mod race;
pub mod render;
pub mod rng;
pub mod save;
pub mod selftest;
pub mod track;
