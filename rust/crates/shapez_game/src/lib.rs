//! Couche Bevy du jeu. La logique vit dans `shapez_sim` ; ce crate ne fait
//! qu'afficher, lire les entrées et piloter le cycle de vie (plan, §3.1 et §3.5).

use bevy::prelude::*;

pub mod menu;
pub mod sim;
pub mod state;

pub use state::AppState;

/// Regroupe tous les plugins du jeu.
#[derive(Debug, Default)]
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((state::AppStatePlugin, sim::SimPlugin, menu::MenuPlugin))
            .add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
