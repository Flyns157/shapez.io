//! Pont entre Bevy et `shapez_sim` : le monde avance dans `FixedUpdate` à 60 Hz.

use bevy::prelude::*;
use shapez_sim::{SimWorld, TICKS_PER_SECOND};

use crate::AppState;

/// Monde simulé, possédé par Bevy comme ressource.
#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct Sim(pub SimWorld);

/// Vitesse de jeu : nombre de ticks exécutés par pas fixe (0 = pause).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameSpeed(pub u32);

impl Default for GameSpeed {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Debug, Default)]
pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(f64::from(TICKS_PER_SECOND)))
            .init_resource::<GameSpeed>()
            .add_systems(OnEnter(AppState::InGame), |mut commands: Commands| {
                commands.insert_resource(Sim::default());
            })
            .add_systems(OnExit(AppState::InGame), |mut commands: Commands| {
                commands.remove_resource::<Sim>();
            })
            .add_systems(FixedUpdate, step_sim.run_if(in_state(AppState::InGame)));
    }
}

/// On ne modifie jamais `dt` : accélérer revient à exécuter plusieurs ticks.
fn step_sim(mut sim: ResMut<Sim>, speed: Res<GameSpeed>) {
    sim.advance(speed.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    fn app_in_game(speed: u32) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, SimPlugin))
            .insert_state(AppState::InGame)
            .insert_resource(GameSpeed(speed));
        app.update();
        app
    }

    #[test]
    fn speed_runs_n_ticks_per_fixed_step() {
        let mut app = app_in_game(4);
        let before = app.world().resource::<Sim>().tick_count();
        app.world_mut().run_schedule(FixedUpdate);
        let after = app.world().resource::<Sim>().tick_count();
        assert_eq!(after - before, 4);
    }

    #[test]
    fn paused_runs_no_tick() {
        let mut app = app_in_game(0);
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(app.world().resource::<Sim>().tick_count(), 0);
    }
}
