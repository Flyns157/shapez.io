//! Types de base partagés par toute la simulation.
//!
//! Cette crate ne dépend pas de Bevy (voir `docs/plan-rust-bevy.md`, §3 et §4).

pub mod grid;

pub use grid::{Direction, Rotation, TilePos};
