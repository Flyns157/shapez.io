//! Port exact du générateur Alea de `src/js/core/rng.js` (Johannes Baagøe, 2010).
//!
//! Le JS calcule en `f64` : on reproduit les mêmes opérations IEEE 754, y compris les
//! conversions `>>> 0` (ToUint32) et `| 0` (ToInt32), pour obtenir **exactement** la
//! même suite de nombres à partir de la même graine. C'est ce qui permet de générer
//! les mêmes cartes que le jeu d'origine. Ce générateur sert à la génération de
//! contenu (carte, formes du freeplay), jamais à la logique par tick.

/// `x >>> 0` en JavaScript, pour un `f64` fini.
fn to_uint32(x: f64) -> u32 {
    let t = x.trunc();
    t.rem_euclid(4_294_967_296.0) as u32
}

/// `x | 0` en JavaScript, pour un `f64` fini.
fn to_int32(x: f64) -> i32 {
    to_uint32(x) as i32
}

const TWO_POW_MINUS_32: f64 = 2.328_306_436_538_696_3e-10;

/// Fonction de hachage `Mash` d'Alea (état interne `n`).
struct Mash {
    n: f64,
}

impl Mash {
    fn new() -> Self {
        Self {
            n: f64::from(0xefc8_249d_u32),
        }
    }

    fn hash(&mut self, data: &str) -> f64 {
        let mut n = self.n;
        // `charCodeAt` : unités UTF-16.
        for unit in data.encode_utf16() {
            n += f64::from(unit);
            let mut h = 0.025_196_032_824_169_38 * n;
            n = f64::from(to_uint32(h));
            h -= n;
            h *= n;
            n = f64::from(to_uint32(h));
            h -= n;
            n += h * 4_294_967_296.0;
        }
        self.n = n;
        f64::from(to_uint32(n)) * TWO_POW_MINUS_32
    }
}

/// Générateur Alea, compatible bit à bit avec `RandomNumberGenerator` du JS.
#[derive(Debug, Clone, PartialEq)]
pub struct Alea {
    s0: f64,
    s1: f64,
    s2: f64,
    c: f64,
}

impl Alea {
    /// `new RandomNumberGenerator(seed)` : la graine est la chaîne passée au JS
    /// (le JS appelle `toString()` sur la graine).
    pub fn new(seed: &str) -> Self {
        let mut mash = Mash::new();
        let mut s0 = mash.hash(" ");
        let mut s1 = mash.hash(" ");
        let mut s2 = mash.hash(" ");
        s0 -= mash.hash(seed);
        if s0 < 0.0 {
            s0 += 1.0;
        }
        s1 -= mash.hash(seed);
        if s1 < 0.0 {
            s1 += 1.0;
        }
        s2 -= mash.hash(seed);
        if s2 < 0.0 {
            s2 += 1.0;
        }
        Self { s0, s1, s2, c: 1.0 }
    }

    /// `next()` : nombre dans `[0, 1[`.
    pub fn next_f64(&mut self) -> f64 {
        let t = 2_091_639.0 * self.s0 + self.c * TWO_POW_MINUS_32;
        self.s0 = self.s1;
        self.s1 = self.s2;
        self.c = f64::from(to_int32(t));
        self.s2 = t - self.c;
        self.s2
    }

    /// `nextIntRange(min, max)` : `Math.floor(next() * (max - min) + min)`.
    ///
    /// Les bornes sont des `f64` car le JS l'appelle parfois avec des demi-entiers.
    pub fn next_int_range(&mut self, min: f64, max: f64) -> i64 {
        (self.next_f64() * (max - min) + min).floor() as i64
    }

    /// `nextRange(min, max)`.
    pub fn next_range(&mut self, min: f64, max: f64) -> f64 {
        self.next_f64() * (max - min) + min
    }

    /// `choice(array)`.
    pub fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let idx = self.next_int_range(0.0, items.len() as f64);
        &items[idx as usize]
    }
}

/// `Math.round` du JavaScript : arrondi vers +∞ pour les demis (différent de `f64::round`).
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_round_rounds_half_up() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(-2.6), -3.0);
    }

    #[test]
    fn int_conversions_match_js() {
        assert_eq!(to_uint32(-1.0), 4_294_967_295);
        assert_eq!(to_uint32(4_294_967_297.7), 1);
        assert_eq!(to_int32(2_147_483_648.0), -2_147_483_648);
    }
}
