//! Propriétés algébriques des opérations sur les formes.

use proptest::prelude::*;
use shapez_core::{Color, Layer, Quadrant, ShapeDef, SubShape};

fn quadrant() -> impl Strategy<Value = Option<Quadrant>> {
    prop::option::of(
        (
            prop::sample::select(SubShape::ALL.to_vec()),
            prop::sample::select(Color::ALL.to_vec()),
        )
            .prop_map(|(sub, color)| Quadrant::new(sub, color)),
    )
}

fn layer() -> impl Strategy<Value = Layer> {
    [quadrant(), quadrant(), quadrant(), quadrant()]
        .prop_filter("couche non vide", |l: &Layer| l.iter().any(Option::is_some))
}

fn shape() -> impl Strategy<Value = ShapeDef> {
    prop::collection::vec(layer(), 1..=4).prop_map(|layers| ShapeDef::from_layers(&layers))
}

proptest! {
    #[test]
    fn short_key_roundtrip(s in shape()) {
        prop_assert_eq!(s.short_key().parse::<ShapeDef>().unwrap(), s);
    }

    #[test]
    fn rotations_compose(s in shape()) {
        prop_assert_eq!(s.rotate_cw().rotate_cw().rotate_cw().rotate_cw(), s);
        prop_assert_eq!(s.rotate_cw().rotate_ccw(), s);
        prop_assert_eq!(s.rotate_cw().rotate_cw(), s.rotate_180());
    }

    #[test]
    fn cut_halves_restack_to_original_when_single_layer(l in layer()) {
        let s = ShapeDef::from_layers(&[l]);
        let [west, east] = s.cut_half();
        prop_assert_eq!(west.stack(&east), s);
    }

    #[test]
    fn stack_never_exceeds_four_layers(a in shape(), b in shape()) {
        prop_assert!(a.stack(&b).layer_count() <= 4);
    }
}
