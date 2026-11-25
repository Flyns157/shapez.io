//! Simulation du jeu, indépendante de Bevy.
//!
//! Le monde avance par ticks fixes ([`TICKS_PER_SECOND`] par seconde) et toute la
//! logique travaille en entiers, pour rester déterministe (plan, §5.1 et §5.2).
//! Les bâtiments, convoyeurs et wires arriveront aux jalons M3 et suivants.

pub mod map;

/// Fréquence fixe de la simulation.
pub const TICKS_PER_SECOND: u32 = 60;

/// État complet de la simulation.
#[derive(Debug, Clone)]
pub struct SimWorld {
    tick: u64,
    /// Carte (ressources générées depuis la graine).
    pub map: map::Map,
}

impl Default for SimWorld {
    fn default() -> Self {
        Self::new(0)
    }
}

impl SimWorld {
    /// Nouveau monde ; la graine est un entier comme `map.seed` dans le JS.
    pub fn new(seed: u32) -> Self {
        Self {
            tick: 0,
            map: map::Map::new(seed),
        }
    }

    /// Numéro du tick courant (0 au démarrage).
    pub fn tick_count(&self) -> u64 {
        self.tick
    }

    /// Fait avancer le monde d'un tick.
    pub fn tick(&mut self) {
        self.tick += 1;
    }

    /// Fait avancer le monde de `n` ticks (vitesse ×N).
    pub fn advance(&mut self, n: u32) {
        for _ in 0..n {
            self.tick();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_by_whole_ticks() {
        let mut world = SimWorld::new(0);
        world.advance(TICKS_PER_SECOND);
        assert_eq!(world.tick_count(), 60);
    }
}
