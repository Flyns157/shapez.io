//! Menu principal minimal et écran de jeu provisoire (affiche le compteur de ticks).

use bevy::{
    prelude::*,
    ui_widgets::{Activate, ActivateOnPress, Button},
};

use crate::{
    AppState,
    sim::{GameSpeed, Sim},
};

#[derive(Debug, Default)]
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_menu)
            .add_observer(start_game)
            .add_systems(OnEnter(AppState::InGame), spawn_hud)
            .add_systems(
                Update,
                (update_hud, game_keys).run_if(in_state(AppState::InGame)),
            );
    }
}

#[derive(Component, Debug)]
struct PlayButton;

#[derive(Component, Debug)]
struct GalleryButton;

#[derive(Component, Debug)]
struct TickText;

fn spawn_menu(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(AppState::MainMenu),
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: px(24),
            ..default()
        },
        children![
            (
                Text::new("shapez-rs"),
                TextFont {
                    font_size: FontSize::Px(48.0),
                    ..default()
                },
            ),
            (
                PlayButton,
                Button,
                ActivateOnPress,
                Node {
                    width: px(180),
                    height: px(60),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
                children![Text::new("Jouer")],
            ),
            (
                GalleryButton,
                Button,
                ActivateOnPress,
                Node {
                    width: px(180),
                    height: px(48),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
                children![Text::new("Formes")],
            ),
        ],
    ));
}

fn start_game(
    activate: On<Activate>,
    play: Query<(), With<PlayButton>>,
    gallery: Query<(), With<GalleryButton>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let target = activate.event_target();
    if play.contains(target) {
        next.set(AppState::InGame);
    } else if gallery.contains(target) {
        next.set(AppState::ShapeGallery);
    }
}

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(AppState::InGame),
        TickText,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn update_hud(sim: Res<Sim>, speed: Res<GameSpeed>, mut text: Single<&mut Text, With<TickText>>) {
    text.0 = format!(
        "Tick {} - vitesse x{} (1/2/4, espace = pause, Echap = menu)",
        sim.tick_count(),
        speed.0
    );
}

fn game_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut speed: ResMut<GameSpeed>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        speed.0 = 1;
    } else if keys.just_pressed(KeyCode::Digit2) {
        speed.0 = 2;
    } else if keys.just_pressed(KeyCode::Digit4) {
        speed.0 = 4;
    } else if keys.just_pressed(KeyCode::Space) {
        speed.0 = if speed.0 == 0 { 1 } else { 0 };
    }
    if keys.just_pressed(KeyCode::Escape) {
        next.set(AppState::MainMenu);
    }
}
