//! Compare les opérations sur les formes au code JS d'origine.
//!
//! Les vecteurs sont produits par `tools/js-oracle/shapes.mjs`. La variable
//! d'environnement `SHAPEZ_GOLDEN_SHAPES` permet de pointer vers un jeu plus grand
//! (par exemple `node tools/js-oracle/shapes.mjs 10000 > /tmp/shapes.json`).

use serde::Deserialize;
use shapez_core::{Color, ShapeDef};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    shape: String,
    other: String,
    cut: [String; 2],
    cut_quad: [String; 4],
    cw: String,
    ccw: String,
    r180: String,
    stack: String,
    paint: String,
    painted: String,
    paint4: [Option<String>; 4],
    painted4: String,
    valid: bool,
    mix: [String; 2],
    mixed: String,
}

fn color(name: &str) -> Color {
    Color::from_js_name(name).unwrap_or_else(|| panic!("couleur inconnue : {name}"))
}

#[test]
fn shapes_match_js_oracle() {
    let path = std::env::var("SHAPEZ_GOLDEN_SHAPES").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/shapes.json").into()
    });
    let data = std::fs::read_to_string(&path).expect("lecture des vecteurs golden");
    let cases: Vec<Case> = serde_json::from_str(&data).expect("JSON valide");
    assert!(!cases.is_empty());

    for (i, c) in cases.iter().enumerate() {
        let ctx = format!("cas {i} ({})", c.shape);
        let shape: ShapeDef = c.shape.parse().expect(&ctx);
        let other: ShapeDef = c.other.parse().expect(&ctx);
        assert!(c.valid, "{ctx}");
        assert_eq!(shape.short_key(), c.shape, "{ctx} : format");

        let [a, b] = shape.cut_half();
        assert_eq!([a.short_key(), b.short_key()], c.cut, "{ctx} : découpe");
        assert_eq!(
            shape.cut_quad().map(|s| s.short_key()),
            c.cut_quad,
            "{ctx} : découpe ×4"
        );
        assert_eq!(
            shape.rotate_cw().short_key(),
            c.cw,
            "{ctx} : rotation horaire"
        );
        assert_eq!(
            shape.rotate_ccw().short_key(),
            c.ccw,
            "{ctx} : rotation anti-horaire"
        );
        assert_eq!(shape.rotate_180().short_key(), c.r180, "{ctx} : demi-tour");
        assert_eq!(
            shape.stack(&other).short_key(),
            c.stack,
            "{ctx} : empilement sur {}",
            c.other
        );
        assert_eq!(
            shape.paint(color(&c.paint)).short_key(),
            c.painted,
            "{ctx} : peinture"
        );
        let colors = c.paint4.clone().map(|o| o.map(|n| color(&n)));
        assert_eq!(
            shape.paint_quadrants(colors).short_key(),
            c.painted4,
            "{ctx} : peinture ×4"
        );
        assert_eq!(
            color(&c.mix[0]).mix(color(&c.mix[1])),
            color(&c.mixed),
            "{ctx} : mélange"
        );
    }
}
