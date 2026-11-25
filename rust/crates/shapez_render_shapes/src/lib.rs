//! Rastérisation des formes, portée de `ShapeDefinition.internalGenerateShapeBuffer`
//! (`src/js/game/shape_definition.js`) vers tiny-skia.
//!
//! Mêmes chemins, même ordre de dessin et mêmes proportions que le Canvas 2D du JS :
//! une forme occupe 23 unités de large, chaque quadrant 10 unités, et chaque couche
//! est réduite de 22 % par rapport à la précédente. Cette crate ne dépend pas de Bevy.

use shapez_core::{Color, ShapeDef, SubShape};
use tiny_skia::{
    FillRule, LineJoin, Paint, PathBuilder, Pixmap, PremultipliedColorU8, Stroke, Transform,
};

/// Couleurs de thème utilisées par le dessin (`THEME.items` dans le JS).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeTheme {
    /// Contour des quadrants, RGBA.
    pub outline: [u8; 4],
    /// Épaisseur du contour, en unités de forme.
    pub outline_width: f32,
    /// Disque de fond derrière la forme, RGBA.
    pub circle_background: [u8; 4],
}

impl ShapeTheme {
    /// `themes/light.json`.
    pub const LIGHT: Self = Self {
        outline: [0x55, 0x57, 0x5a, 0xff],
        outline_width: 0.75,
        circle_background: [40, 50, 65, 26], // rgba(40, 50, 65, 0.1)
    };

    /// `themes/dark.json`.
    pub const DARK: Self = Self {
        outline: [0x11, 0x14, 0x18, 0xff],
        outline_width: 0.75,
        circle_background: [20, 30, 40, 77], // rgba(20, 30, 40, 0.3)
    };
}

impl Default for ShapeTheme {
    fn default() -> Self {
        Self::LIGHT
    }
}

const QUADRANT_SIZE: f32 = 10.0;
const QUADRANT_HALF: f32 = QUADRANT_SIZE / 2.0;
/// Constante de Bézier pour approcher un quart de cercle.
const KAPPA: f32 = 0.552_284_8;

/// Centre de chaque quadrant, dans l'ordre TR, BR, BL, TL (`arrayQuadrantIndexToOffset`).
const QUADRANT_OFFSETS: [(f32, f32); 4] = [(1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0)];

/// Dessine la forme dans une image carrée de `size` pixels (alpha prémultiplié).
pub fn render_pixmap(def: &ShapeDef, size: u32, theme: &ShapeTheme) -> Pixmap {
    let mut pixmap = Pixmap::new(size.max(1), size.max(1)).expect("taille non nulle");
    let s = size as f32;
    let base = Transform::from_translate(s / 2.0, s / 2.0).pre_scale(s / 23.0, s / 23.0);

    let mut paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };

    let [r, g, b, a] = theme.circle_background;
    paint.set_color_rgba8(r, g, b, a);
    if let Some(circle) = PathBuilder::from_circle(0.0, 0.0, QUADRANT_SIZE * 1.15) {
        pixmap.fill_path(&circle, &paint, FillRule::Winding, base, None);
    }

    let stroke = Stroke {
        width: theme.outline_width,
        // Valeurs par défaut du Canvas 2D.
        miter_limit: 10.0,
        line_join: LineJoin::Miter,
        ..Stroke::default()
    };
    let mut outline = paint.clone();
    let [r, g, b, a] = theme.outline;
    outline.set_color_rgba8(r, g, b, a);

    for (layer_idx, layer) in def.layers().iter().enumerate() {
        let layer_scale = (0.9 - layer_idx as f32 * 0.22).max(0.1);
        for (q, quad) in layer.iter().enumerate() {
            let Some(quad) = quad else { continue };
            let (ox, oy) = QUADRANT_OFFSETS[q];
            let transform = base
                .pre_translate(ox * QUADRANT_HALF, oy * QUADRANT_HALF)
                .pre_rotate(q as f32 * 90.0);
            let Some(path) = sub_shape_path(quad.sub, layer_scale) else {
                continue;
            };
            let [r, g, b] = quad.color.rgb();
            paint.set_color_rgba8(r, g, b, 0xff);
            pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
            pixmap.stroke_path(&path, &outline, &stroke, transform, None);
        }
    }
    pixmap
}

/// Chemin d'une sous-forme dans le repère local du quadrant (avant rotation).
fn sub_shape_path(sub: SubShape, layer_scale: f32) -> Option<tiny_skia::Path> {
    let dims = QUADRANT_SIZE * layer_scale;
    let (ox, oy) = (-QUADRANT_HALF, QUADRANT_HALF - dims);
    let inwards = dims * 0.4;
    let mut pb = PathBuilder::new();
    match sub {
        SubShape::Rect => {
            pb.move_to(ox, oy);
            pb.line_to(ox + dims, oy);
            pb.line_to(ox + dims, oy + dims);
            pb.line_to(ox, oy + dims);
        }
        SubShape::Star => {
            pb.move_to(ox, oy + inwards);
            pb.line_to(ox + dims, oy);
            pb.line_to(ox + dims - inwards, oy + dims);
            pb.line_to(ox, oy + dims);
        }
        SubShape::Windmill => {
            pb.move_to(ox, oy + inwards);
            pb.line_to(ox + dims, oy);
            pb.line_to(ox + dims, oy + dims);
            pb.line_to(ox, oy + dims);
        }
        SubShape::Circle => {
            // arc(cx, cy, r, -π/2, 0) : quart de cercle de (cx, cy - r) à (cx + r, cy).
            let (cx, cy, r) = (-QUADRANT_HALF, QUADRANT_HALF, dims);
            pb.move_to(cx, cy);
            pb.line_to(cx, cy - r);
            pb.cubic_to(cx + KAPPA * r, cy - r, cx + r, cy - KAPPA * r, cx + r, cy);
        }
    }
    pb.close();
    pb.finish()
}

/// Dessine la forme et renvoie des pixels RGBA8 en alpha *non* prémultiplié,
/// prêts pour une texture (`size * size * 4` octets).
pub fn render_rgba(def: &ShapeDef, size: u32, theme: &ShapeTheme) -> Vec<u8> {
    let pixmap = render_pixmap(def, size, theme);
    pixmap
        .pixels()
        .iter()
        .flat_map(|p: &PremultipliedColorU8| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

/// Couleur unie d'un item couleur, dessinée comme un disque (`ColorItem` du JS).
pub fn render_color_rgba(color: Color, size: u32, theme: &ShapeTheme) -> Vec<u8> {
    let mut pixmap = Pixmap::new(size.max(1), size.max(1)).expect("taille non nulle");
    let s = size as f32;
    let transform = Transform::from_translate(s / 2.0, s / 2.0).pre_scale(s / 23.0, s / 23.0);
    let mut paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    let [r, g, b] = color.rgb();
    paint.set_color_rgba8(r, g, b, 0xff);
    if let Some(circle) = PathBuilder::from_circle(0.0, 0.0, QUADRANT_SIZE * 0.6) {
        pixmap.fill_path(&circle, &paint, FillRule::Winding, transform, None);
        let mut outline = paint.clone();
        let [r, g, b, a] = theme.outline;
        outline.set_color_rgba8(r, g, b, a);
        let stroke = Stroke {
            width: theme.outline_width,
            ..Stroke::default()
        };
        pixmap.stroke_path(&circle, &outline, &stroke, transform, None);
    }
    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(rgba: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * size + x) * 4) as usize;
        rgba[i..i + 4].try_into().unwrap()
    }

    #[test]
    fn quadrants_land_where_expected() {
        // Un seul rectangle rouge en haut à droite.
        let def: ShapeDef = "Rr------".parse().unwrap();
        let size = 92;
        let rgba = render_rgba(&def, size, &ShapeTheme::LIGHT);
        assert_eq!(rgba.len(), (size * size * 4) as usize);
        // Centre du quadrant haut-droite : rouge opaque.
        assert_eq!(pixel(&rgba, size, 66, 26), [0xff, 0x66, 0x6a, 0xff]);
        // Centre du quadrant bas-gauche : seulement le disque de fond translucide.
        assert_eq!(pixel(&rgba, size, 26, 66)[3], 26);
        // Coin de l'image : transparent.
        assert_eq!(pixel(&rgba, size, 0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn upper_layers_are_smaller() {
        // La couche 2 (échelle 0,68) laisse voir la couche 1 sur son bord extérieur.
        let def: ShapeDef = "Rr------:Rb------".parse().unwrap();
        let size = 230; // 10 px par unité
        let rgba = render_rgba(&def, size, &ShapeTheme::LIGHT);
        // Près du coin extérieur du quadrant (x≈+9, y≈-9) : couche du bas, rouge.
        assert_eq!(
            &pixel(&rgba, size, 115 + 85, 115 - 85)[..3],
            &[0xff, 0x66, 0x6a]
        );
        // Près du centre du jeu (x≈+2, y≈-2) : couche du haut, bleue.
        assert_eq!(
            &pixel(&rgba, size, 115 + 20, 115 - 20)[..3],
            &[0x66, 0xa7, 0xff]
        );
    }

    #[test]
    fn every_sub_shape_renders() {
        for sub in SubShape::ALL {
            let def = ShapeDef::simple([Some(sub); 4], Color::Green);
            let rgba = render_rgba(&def, 64, &ShapeTheme::DARK);
            assert!(
                rgba.chunks(4).any(|p| p == [0x78, 0xff, 0x66, 0xff]),
                "{sub:?}"
            );
        }
    }
}
