//! Items transportés par les convoyeurs (`src/js/game/items/`).

use crate::{Color, ShapeId};

/// Un item : valeur `Copy` de 8 octets, rangée telle quelle dans les convoyeurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Item {
    Shape(ShapeId),
    Color(Color),
    /// Signal booléen de la couche wires.
    Bool(bool),
}

const _: () = assert!(std::mem::size_of::<Item>() == 8);
