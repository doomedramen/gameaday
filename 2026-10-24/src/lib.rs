//! Cragswing - a rope-swinging platformer roguelite.
//!
//! All simulation, rendering and sound synthesis live in this library; the
//! binary is a thin window/audio adapter.
#![allow(
    clippy::too_many_arguments,
    clippy::approx_constant,
    clippy::type_complexity,
    clippy::needless_range_loop,
    clippy::manual_is_multiple_of,
    clippy::collapsible_if,
    clippy::collapsible_match
)]
pub mod app;
pub mod audio;
pub mod boss;
pub mod bot;
pub mod game;
pub mod level;
pub mod rng;
pub mod run;
pub mod save;
pub mod selftest;
pub mod sim;
pub mod stagegen;
pub mod gfx {
    pub mod bg;
    pub mod canvas;
    pub mod fx;
    pub mod palette;
    pub mod scene;
    pub mod sprites;
    pub mod tiles;
    pub mod ui;
}
