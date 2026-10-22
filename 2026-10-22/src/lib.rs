//! Lanternfall - a turn-based tactics roguelite of light and shadow.
//!
//! Everything here is engine-free: the simulation (`map`, `units`, `battle`,
//! `ai`, `run`), persistence (`save`), the playtest bot (`bot`), a pure
//! software renderer that draws into a `u32` framebuffer (`gfx`), and a
//! sample-producing synthesiser (`audio`). `main.rs` only wires these to a
//! window and an audio device.

pub mod ai;
pub mod app;
pub mod audio;
pub mod battle;
pub mod bot;
pub mod gfx;
pub mod map;
pub mod rng;
pub mod run;
pub mod save;
pub mod selftest;
pub mod units;
