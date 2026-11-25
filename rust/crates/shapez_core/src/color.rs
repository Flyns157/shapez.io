//! Couleurs des formes et table de mélange (`src/js/game/colors.js`).

use std::fmt;

/// Une des huit couleurs du jeu.
///
/// La valeur discriminante est le champ de bits rouge/vert/bleu du JS
/// (`bitfieldToColor`) : le mélange de deux couleurs est un simple `OU`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Color {
    Uncolored = 0b000,
    Red = 0b001,
    Green = 0b010,
    Yellow = 0b011,
    Blue = 0b100,
    Purple = 0b101,
    Cyan = 0b110,
    White = 0b111,
}

impl Color {
    /// Ordre de déclaration de `enumColors` dans le JS.
    pub const ALL: [Self; 8] = [
        Self::Red,
        Self::Green,
        Self::Blue,
        Self::Yellow,
        Self::Purple,
        Self::Cyan,
        Self::White,
        Self::Uncolored,
    ];

    const fn from_bits(bits: u8) -> Self {
        match bits & 0b111 {
            0b000 => Self::Uncolored,
            0b001 => Self::Red,
            0b010 => Self::Green,
            0b011 => Self::Yellow,
            0b100 => Self::Blue,
            0b101 => Self::Purple,
            0b110 => Self::Cyan,
            _ => Self::White,
        }
    }

    /// Résultat du mélangeur (`enumColorMixingResults`).
    pub const fn mix(self, other: Self) -> Self {
        Self::from_bits(self as u8 | other as u8)
    }

    /// Lettre du code court (`enumColorToShortcode`).
    pub const fn shortcode(self) -> char {
        match self {
            Self::Red => 'r',
            Self::Green => 'g',
            Self::Blue => 'b',
            Self::Yellow => 'y',
            Self::Purple => 'p',
            Self::Cyan => 'c',
            Self::White => 'w',
            Self::Uncolored => 'u',
        }
    }

    pub const fn from_shortcode(c: char) -> Option<Self> {
        Some(match c {
            'r' => Self::Red,
            'g' => Self::Green,
            'b' => Self::Blue,
            'y' => Self::Yellow,
            'p' => Self::Purple,
            'c' => Self::Cyan,
            'w' => Self::White,
            'u' => Self::Uncolored,
            _ => return None,
        })
    }

    /// Nom utilisé par le JS (`enumColors`) et dans ses sauvegardes.
    pub const fn js_name(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Yellow => "yellow",
            Self::Purple => "purple",
            Self::Cyan => "cyan",
            Self::White => "white",
            Self::Uncolored => "uncolored",
        }
    }

    pub fn from_js_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.js_name() == name)
    }

    /// Couleur d'affichage en RGB (`enumColorsToHexCode`).
    pub const fn rgb(self) -> [u8; 3] {
        match self {
            Self::Red => [0xff, 0x66, 0x6a],
            Self::Green => [0x78, 0xff, 0x66],
            Self::Blue => [0x66, 0xa7, 0xff],
            Self::Yellow => [0xfc, 0xf5, 0x2a],
            Self::Purple => [0xdd, 0x66, 0xff],
            Self::Cyan => [0x00, 0xfc, 0xff],
            Self::White => [0xff, 0xff, 0xff],
            Self::Uncolored => [0xaa, 0xaa, 0xaa],
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.js_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixing_matches_js_table() {
        use Color::*;
        assert_eq!(Red.mix(Green), Yellow);
        assert_eq!(Red.mix(Blue), Purple);
        assert_eq!(Blue.mix(Green), Cyan);
        assert_eq!(Yellow.mix(Blue), White);
        assert_eq!(Uncolored.mix(Red), Red);
        assert_eq!(White.mix(Red), White);
        for a in Color::ALL {
            assert_eq!(a.mix(a), a);
            for b in Color::ALL {
                assert_eq!(a.mix(b), b.mix(a));
            }
        }
    }

    #[test]
    fn shortcodes_roundtrip() {
        for c in Color::ALL {
            assert_eq!(Color::from_shortcode(c.shortcode()), Some(c));
            assert_eq!(Color::from_js_name(c.js_name()), Some(c));
        }
        assert_eq!(Color::from_shortcode('-'), None);
    }
}
