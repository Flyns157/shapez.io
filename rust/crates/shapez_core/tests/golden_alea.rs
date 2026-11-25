//! Le port d'Alea doit produire exactement (au bit près) la suite du JS.
//! Vecteurs : `tools/js-oracle/rng.mjs`.

use serde::Deserialize;
use shapez_core::Alea;

#[derive(Deserialize)]
struct Case {
    seed: String,
    values: Vec<f64>,
    ints: Vec<i64>,
}

#[test]
fn alea_matches_js_bit_for_bit() {
    let data = include_str!("golden/alea.json");
    let cases: Vec<Case> = serde_json::from_str(data).unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let mut rng = Alea::new(&case.seed);
        for (i, &expected) in case.values.iter().enumerate() {
            let got = rng.next_f64();
            assert_eq!(
                got.to_bits(),
                expected.to_bits(),
                "graine {:?}, tirage {i}",
                case.seed
            );
        }
        for (i, &expected) in case.ints.iter().enumerate() {
            assert_eq!(
                rng.next_int_range(-3.5, 7.5),
                expected,
                "graine {:?}, entier {i}",
                case.seed
            );
        }
    }
}
