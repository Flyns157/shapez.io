//! Définition des formes et opérations pures (`src/js/game/shape_definition.js`).
//!
//! Une forme a de 0 à 4 couches, de bas en haut. Chaque couche a 4 quadrants dans
//! l'ordre du JS : haut-droite (0), bas-droite (1), bas-gauche (2), haut-gauche (3).
//! Le code court (`RuCw--Cw:----Ru--`) sert de clé unique, comme `getHash()`.

use std::{fmt, str::FromStr};

use crate::Color;

/// Nombre maximal de couches (`mergedLayers.splice(4)` dans le JS).
pub const MAX_LAYERS: usize = 4;

/// Index des quadrants, comme les constantes `TOP_RIGHT`… du JS.
pub const TOP_RIGHT: usize = 0;
pub const BOTTOM_RIGHT: usize = 1;
pub const BOTTOM_LEFT: usize = 2;
pub const TOP_LEFT: usize = 3;

/// Sous-forme d'un quadrant (`enumSubShape`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SubShape {
    Rect,
    Circle,
    Star,
    Windmill,
}

impl SubShape {
    pub const ALL: [Self; 4] = [Self::Rect, Self::Circle, Self::Star, Self::Windmill];

    pub const fn shortcode(self) -> char {
        match self {
            Self::Rect => 'R',
            Self::Circle => 'C',
            Self::Star => 'S',
            Self::Windmill => 'W',
        }
    }

    pub const fn from_shortcode(c: char) -> Option<Self> {
        Some(match c {
            'R' => Self::Rect,
            'C' => Self::Circle,
            'S' => Self::Star,
            'W' => Self::Windmill,
            _ => return None,
        })
    }
}

/// Contenu d'un quadrant non vide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Quadrant {
    pub sub: SubShape,
    pub color: Color,
}

impl Quadrant {
    pub const fn new(sub: SubShape, color: Color) -> Self {
        Self { sub, color }
    }
}

/// Une couche : quatre quadrants, éventuellement vides.
pub type Layer = [Option<Quadrant>; 4];

const EMPTY_LAYER: Layer = [None; 4];

/// Définition d'une forme, type valeur compact (`Copy`).
///
/// Invariant : les couches au-delà de `len` sont vides, ce qui rend `Eq` et `Hash`
/// cohérents avec le code court.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ShapeDef {
    layers: [Layer; MAX_LAYERS],
    len: u8,
}

/// Erreur de lecture d'un code court.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseShapeError {
    /// Aucune couche, ou plus de quatre.
    LayerCount(usize),
    /// Une couche ne fait pas exactement 8 caractères.
    LayerLength { layer: usize },
    /// Sous-forme ou couleur inconnue, ou quadrant vide mal écrit.
    InvalidQuadrant { layer: usize, quadrant: usize },
    /// Couche sans aucun quadrant.
    EmptyLayer { layer: usize },
}

impl fmt::Display for ParseShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LayerCount(n) => write!(f, "nombre de couches invalide : {n}"),
            Self::LayerLength { layer } => write!(f, "couche {layer} : 8 caractères attendus"),
            Self::InvalidQuadrant { layer, quadrant } => {
                write!(f, "couche {layer}, quadrant {quadrant} invalide")
            }
            Self::EmptyLayer { layer } => write!(f, "couche {layer} vide"),
        }
    }
}

impl std::error::Error for ParseShapeError {}

impl ShapeDef {
    /// Forme sans aucune couche (résultat possible d'une découpe).
    pub const EMPTY: Self = Self {
        layers: [EMPTY_LAYER; MAX_LAYERS],
        len: 0,
    };

    /// Construit une forme à partir de couches, de bas en haut.
    ///
    /// Les couches au-delà de [`MAX_LAYERS`] sont ignorées.
    pub fn from_layers(layers: &[Layer]) -> Self {
        let mut def = Self::EMPTY;
        for layer in layers.iter().take(MAX_LAYERS) {
            def.push_layer(*layer);
        }
        def
    }

    /// Forme d'une seule couche (`getDefinitionFromSimpleShapes`).
    pub fn simple(subs: [Option<SubShape>; 4], color: Color) -> Self {
        Self::from_layers(&[subs.map(|s| s.map(|sub| Quadrant::new(sub, color)))])
    }

    fn push_layer(&mut self, layer: Layer) {
        debug_assert!((self.len as usize) < MAX_LAYERS);
        self.layers[self.len as usize] = layer;
        self.len += 1;
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers[..self.len as usize]
    }

    pub fn layer_count(&self) -> usize {
        self.len as usize
    }

    /// `isEntirelyEmpty()`.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Lit un code court en appliquant les règles de `isValidShortKeyInternal`.
    pub fn from_short_key(key: &str) -> Result<Self, ParseShapeError> {
        let parts: Vec<&str> = key.split(':').collect();
        if parts.is_empty() || parts.len() > MAX_LAYERS {
            return Err(ParseShapeError::LayerCount(parts.len()));
        }
        let mut def = Self::EMPTY;
        for (layer_idx, text) in parts.into_iter().enumerate() {
            let chars: Vec<char> = text.chars().collect();
            if chars.len() != 8 {
                return Err(ParseShapeError::LayerLength { layer: layer_idx });
            }
            let mut layer = EMPTY_LAYER;
            for (q, slot) in layer.iter_mut().enumerate() {
                let (s, c) = (chars[q * 2], chars[q * 2 + 1]);
                let invalid = ParseShapeError::InvalidQuadrant {
                    layer: layer_idx,
                    quadrant: q,
                };
                *slot = match (s, c) {
                    ('-', '-') => None,
                    _ => {
                        let sub = SubShape::from_shortcode(s).ok_or(invalid.clone())?;
                        let color = Color::from_shortcode(c).ok_or(invalid)?;
                        Some(Quadrant::new(sub, color))
                    }
                };
            }
            if layer.iter().all(Option::is_none) {
                return Err(ParseShapeError::EmptyLayer { layer: layer_idx });
            }
            def.push_layer(layer);
        }
        Ok(def)
    }

    /// Code court (`getHash()`). Une forme vide donne la chaîne vide, comme le JS.
    pub fn short_key(&self) -> String {
        self.to_string()
    }

    /// `cloneFilteredByQuadrants` : garde les quadrants indiqués et retire les couches
    /// devenues vides (y compris au milieu de la pile, comme le JS).
    pub fn filtered_by_quadrants(&self, keep: [bool; 4]) -> Self {
        let mut out = Self::EMPTY;
        for layer in self.layers() {
            let mut filtered = EMPTY_LAYER;
            for q in 0..4 {
                if keep[q] {
                    filtered[q] = layer[q];
                }
            }
            if filtered.iter().any(Option::is_some) {
                out.push_layer(filtered);
            }
        }
        out
    }

    /// Découpeur (`shapeActionCutHalf`) : renvoie `[quadrants 2 et 3, quadrants 0 et 1]`,
    /// soit la moitié gauche puis la moitié droite, dans l'ordre du JS.
    pub fn cut_half(&self) -> [Self; 2] {
        [
            self.filtered_by_quadrants([false, false, true, true]),
            self.filtered_by_quadrants([true, true, false, false]),
        ]
    }

    /// Découpeur quadruple (`shapeActionCutQuad`) : un résultat par quadrant 0 à 3.
    pub fn cut_quad(&self) -> [Self; 4] {
        std::array::from_fn(|q| {
            let mut keep = [false; 4];
            keep[q] = true;
            self.filtered_by_quadrants(keep)
        })
    }

    fn map_layers(&self, f: impl Fn(Layer) -> Layer) -> Self {
        let mut out = *self;
        for layer in &mut out.layers[..self.len as usize] {
            *layer = f(*layer);
        }
        out
    }

    /// Rotation horaire (`cloneRotateCW`).
    pub fn rotate_cw(&self) -> Self {
        self.map_layers(|mut l| {
            l.rotate_right(1);
            l
        })
    }

    /// Rotation anti-horaire (`cloneRotateCCW`).
    pub fn rotate_ccw(&self) -> Self {
        self.map_layers(|mut l| {
            l.rotate_left(1);
            l
        })
    }

    /// Demi-tour (`cloneRotate180`).
    pub fn rotate_180(&self) -> Self {
        self.map_layers(|mut l| {
            l.rotate_left(2);
            l
        })
    }

    /// Empileur (`cloneAndStackWith`) : pose `top` sur `self`.
    ///
    /// La forme du dessus descend jusqu'à toucher la forme du dessous, puis le résultat
    /// est tronqué à 4 couches. Le JS refuse d'empiler une forme vide ; ici, empiler sur
    /// (ou avec) une forme vide renvoie simplement l'autre forme.
    pub fn stack(&self, top: &Self) -> Self {
        if self.is_empty() {
            return *top;
        }
        if top.is_empty() {
            return *self;
        }

        let mut bottom_highest = [-1i32; 4];
        for (idx, layer) in self.layers().iter().enumerate() {
            for q in 0..4 {
                if layer[q].is_some() {
                    bottom_highest[q] = bottom_highest[q].max(idx as i32);
                }
            }
        }
        let mut top_lowest = [MAX_LAYERS as i32; 4];
        for (idx, layer) in top.layers().iter().enumerate().rev() {
            for q in 0..4 {
                if layer[q].is_some() {
                    top_lowest[q] = top_lowest[q].min(idx as i32);
                }
            }
        }

        let smallest_gap = (0..4)
            .map(|q| top_lowest[q] - bottom_highest[q])
            .min()
            .unwrap_or(1);
        let merge_at = (1 - smallest_gap).max(0) as usize;

        // Jusqu'à 8 couches avant la troncature finale.
        let mut merged = [EMPTY_LAYER; MAX_LAYERS * 2];
        merged[..self.layer_count()].copy_from_slice(self.layers());
        let total = self.layer_count().max(merge_at + top.layer_count());
        for (idx, top_layer) in top.layers().iter().enumerate() {
            let target = &mut merged[merge_at + idx];
            for q in 0..4 {
                debug_assert!(
                    !(target[q].is_some() && top_layer[q].is_some()),
                    "empilement : un quadrant serait perdu"
                );
                target[q] = target[q].or(top_layer[q]);
            }
        }
        Self::from_layers(&merged[..total])
    }

    /// Peintre (`cloneAndPaintWith`) : toute la forme prend la couleur donnée.
    pub fn paint(&self, color: Color) -> Self {
        self.map_layers(|l| l.map(|q| q.map(|q| Quadrant { color, ..q })))
    }

    /// Peintre quadruple (`cloneAndPaintWith4Colors`) : `None` laisse le quadrant intact.
    pub fn paint_quadrants(&self, colors: [Option<Color>; 4]) -> Self {
        self.map_layers(|l| {
            std::array::from_fn(|q| {
                l[q].map(|quad| Quadrant {
                    color: colors[q].unwrap_or(quad.color),
                    ..quad
                })
            })
        })
    }
}

impl fmt::Display for ShapeDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, layer) in self.layers().iter().enumerate() {
            if idx > 0 {
                f.write_str(":")?;
            }
            for quad in layer {
                match quad {
                    Some(q) => write!(f, "{}{}", q.sub.shortcode(), q.color.shortcode())?,
                    None => f.write_str("--")?,
                }
            }
        }
        Ok(())
    }
}

impl fmt::Debug for ShapeDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ShapeDef({self})")
    }
}

impl FromStr for ShapeDef {
    type Err = ParseShapeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_short_key(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(key: &str) -> ShapeDef {
        key.parse().unwrap()
    }

    #[test]
    fn parse_and_format_roundtrip() {
        for key in [
            "CuCuCuCu",
            "RuCw--Cw:----Ru--",
            "Sr------:--Sg----:----Sb--:------Sy",
        ] {
            assert_eq!(s(key).short_key(), key);
        }
        assert_eq!(ShapeDef::EMPTY.short_key(), "");
    }

    #[test]
    fn rejects_invalid_keys() {
        assert!(ShapeDef::from_short_key("").is_err());
        assert!(ShapeDef::from_short_key("CuCuCu").is_err());
        assert!(ShapeDef::from_short_key("--------").is_err());
        assert!(ShapeDef::from_short_key("Cu-uCuCu").is_err());
        assert!(ShapeDef::from_short_key("CxCuCuCu").is_err());
        assert!(ShapeDef::from_short_key("Cu------:Cu------:Cu------:Cu------:Cu------").is_err());
    }

    #[test]
    fn rotations() {
        let shape = s("RuCw--Cw:----Ru--");
        assert_eq!(shape.rotate_cw().short_key(), "CwRuCw--:------Ru");
        assert_eq!(shape.rotate_ccw().short_key(), "Cw--CwRu:--Ru----");
        assert_eq!(shape.rotate_180().short_key(), "--CwRuCw:Ru------");
    }

    #[test]
    fn cut_removes_empty_layers() {
        let [west, east] = s("CuCu----:----CuCu").cut_half();
        assert_eq!(west.short_key(), "----CuCu");
        assert_eq!(east.short_key(), "CuCu----");
        let [q0, ..] = s("----CuCu").cut_quad();
        assert!(q0.is_empty());
    }

    #[test]
    fn stack_drops_to_lowest_free_layer() {
        assert_eq!(s("CuCu----").stack(&s("----RuRu")).short_key(), "CuCuRuRu");
        assert_eq!(
            s("CuCuCuCu").stack(&s("RuRuRuRu")).short_key(),
            "CuCuCuCu:RuRuRuRu"
        );
        let tall = s("Cu------:Cu------:Cu------:Cu------");
        assert_eq!(tall.stack(&s("Ru------")).short_key(), tall.short_key());
    }

    #[test]
    fn painting() {
        assert_eq!(s("CuRu----").paint(Color::Red).short_key(), "CrRr----");
        let painted =
            s("CuCuCuCu").paint_quadrants([Some(Color::Red), None, Some(Color::Blue), None]);
        assert_eq!(painted.short_key(), "CrCuCbCu");
    }
}
