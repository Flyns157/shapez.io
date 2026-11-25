//! Types de base partagés par toute la simulation : grille, formes, couleurs, items.
//!
//! Cette crate ne dépend pas de Bevy (voir `docs/plan-rust-bevy.md`, §3 et §4).

pub mod color;
pub mod grid;
pub mod item;
pub mod registry;
pub mod shape;

pub use color::Color;
pub use grid::{Direction, Rotation, TilePos};
pub use item::Item;
pub use registry::{ShapeId, ShapeRegistry};
pub use shape::{Layer, ParseShapeError, Quadrant, ShapeDef, SubShape};
