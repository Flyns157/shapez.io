//! États de haut niveau de l'application.

use bevy::prelude::*;

/// `Boot → Loading → MainMenu → InGame`.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Boot,
    Loading,
    MainMenu,
    InGame,
}

#[derive(Debug, Default)]
pub struct AppStatePlugin;

impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(
                OnEnter(AppState::Boot),
                |mut next: ResMut<NextState<AppState>>| {
                    next.set(AppState::Loading);
                },
            )
            // Rien à charger pour l'instant : les collections d'assets arrivent avec M1/M2.
            .add_systems(
                OnEnter(AppState::Loading),
                |mut next: ResMut<NextState<AppState>>| {
                    next.set(AppState::MainMenu);
                },
            );

        #[cfg(feature = "dev")]
        app.add_systems(Update, bevy::dev_tools::states::log_transitions::<AppState>);
    }
}
