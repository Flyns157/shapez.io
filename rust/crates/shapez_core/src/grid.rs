//! Géométrie de grille : positions de tuiles, directions et rotations par quart de tour.
//!
//! Conventions reprises de `src/js/core/vector.js` : l'axe Y pointe vers le bas,
//! `Top` vaut donc `(0, -1)`, et les angles se comptent dans le sens horaire
//! (`Top` = 0°, `Right` = 90°, `Bottom` = 180°, `Left` = 270°).

use std::ops::{Add, Sub};

/// Position entière d'une tuile sur la carte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

impl TilePos {
    pub const ZERO: Self = Self::new(0, 0);

    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Tuile voisine dans la direction donnée.
    pub const fn step(self, dir: Direction) -> Self {
        let (dx, dy) = dir.offset();
        Self::new(self.x + dx, self.y + dy)
    }

    /// Fait tourner un décalage local autour de l'origine.
    pub const fn rotated(self, rot: Rotation) -> Self {
        match rot {
            Rotation::R0 => self,
            Rotation::R90 => Self::new(-self.y, self.x),
            Rotation::R180 => Self::new(-self.x, -self.y),
            Rotation::R270 => Self::new(self.y, -self.x),
        }
    }
}

impl Add for TilePos {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for TilePos {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

/// Une des quatre directions cardinales (`enumDirection` du JS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Direction {
    Top,
    Right,
    Bottom,
    Left,
}

impl Direction {
    /// Ordre de `arrayAllDirections` dans le JS.
    pub const ALL: [Self; 4] = [Self::Top, Self::Right, Self::Bottom, Self::Left];

    /// Décalage `(dx, dy)` d'une tuile, Y vers le bas.
    pub const fn offset(self) -> (i32, i32) {
        match self {
            Self::Top => (0, -1),
            Self::Right => (1, 0),
            Self::Bottom => (0, 1),
            Self::Left => (-1, 0),
        }
    }

    pub const fn inverted(self) -> Self {
        self.rotated(Rotation::R180)
    }

    /// Angle de la direction (`enumDirectionToAngle`).
    pub const fn to_rotation(self) -> Rotation {
        match self {
            Self::Top => Rotation::R0,
            Self::Right => Rotation::R90,
            Self::Bottom => Rotation::R180,
            Self::Left => Rotation::R270,
        }
    }

    /// Équivalent de `Vector.transformDirectionFromMultipleOf90`.
    pub const fn rotated(self, rot: Rotation) -> Self {
        Rotation::from_quarter_turns(self.to_rotation().quarter_turns() + rot.quarter_turns())
            .to_direction()
    }
}

/// Rotation par quart de tour, dans le sens horaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Rotation {
    #[default]
    R0,
    R90,
    R180,
    R270,
}

impl Rotation {
    pub const ALL: [Self; 4] = [Self::R0, Self::R90, Self::R180, Self::R270];

    /// Nombre de quarts de tour (0 à 3).
    pub const fn quarter_turns(self) -> u8 {
        self as u8
    }

    /// Construit une rotation depuis un nombre de quarts de tour (modulo 4).
    pub const fn from_quarter_turns(n: u8) -> Self {
        match n % 4 {
            0 => Self::R0,
            1 => Self::R90,
            2 => Self::R180,
            _ => Self::R270,
        }
    }

    /// Angle en degrés, comme dans les sauvegardes JS.
    pub const fn degrees(self) -> u16 {
        self.quarter_turns() as u16 * 90
    }

    /// Lit un angle en degrés multiple de 90 (360 accepté comme 0).
    pub const fn from_degrees(deg: u16) -> Option<Self> {
        if !deg.is_multiple_of(90) || deg > 360 {
            return None;
        }
        Some(Self::from_quarter_turns((deg / 90) as u8))
    }

    pub const fn rotate_cw(self) -> Self {
        Self::from_quarter_turns(self.quarter_turns() + 1)
    }

    pub const fn rotate_ccw(self) -> Self {
        Self::from_quarter_turns(self.quarter_turns() + 3)
    }

    pub const fn then(self, other: Self) -> Self {
        Self::from_quarter_turns(self.quarter_turns() + other.quarter_turns())
    }

    const fn to_direction(self) -> Direction {
        match self {
            Self::R0 => Direction::Top,
            Self::R90 => Direction::Right,
            Self::R180 => Direction::Bottom,
            Self::R270 => Direction::Left,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_rotation_matches_js() {
        // Vector.transformDirectionFromMultipleOf90(top/right, angle)
        assert_eq!(Direction::Top.rotated(Rotation::R90), Direction::Right);
        assert_eq!(Direction::Top.rotated(Rotation::R270), Direction::Left);
        assert_eq!(Direction::Right.rotated(Rotation::R90), Direction::Bottom);
        assert_eq!(Direction::Left.rotated(Rotation::R180), Direction::Right);
    }

    #[test]
    fn rotated_offset_matches_rotated_direction() {
        for dir in Direction::ALL {
            for rot in Rotation::ALL {
                let p = TilePos::ZERO.step(dir).rotated(rot);
                assert_eq!(p, TilePos::ZERO.step(dir.rotated(rot)), "{dir:?} {rot:?}");
            }
        }
    }

    #[test]
    fn rotation_algebra() {
        for r in Rotation::ALL {
            assert_eq!(r.rotate_cw().rotate_ccw(), r);
            assert_eq!(r.rotate_cw().rotate_cw().rotate_cw().rotate_cw(), r);
            assert_eq!(Rotation::from_degrees(r.degrees()), Some(r));
        }
        assert_eq!(Rotation::from_degrees(360), Some(Rotation::R0));
        assert_eq!(Rotation::from_degrees(45), None);
    }

    #[test]
    fn inverted() {
        for dir in Direction::ALL {
            assert_eq!(dir.inverted().inverted(), dir);
            let (dx, dy) = dir.offset();
            assert_eq!(dir.inverted().offset(), (-dx, -dy));
        }
    }
}
