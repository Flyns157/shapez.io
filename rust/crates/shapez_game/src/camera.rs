//! Caméra de jeu : déplacement (clavier, glisser), zoom vers le curseur
//! (port simplifié de `src/js/game/camera.js`).

use bevy::{
    input::mouse::{AccumulatedMouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
};

use crate::AppState;

/// Taille d'une tuile en unités monde (`globalConfig.tileSize`).
pub const TILE_SIZE: f32 = 32.0;
/// Bornes du zoom (`minZoomLevel`, `maxZoomLevel`).
pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 3.0;
/// Sous ce zoom, on passe en vue d'ensemble (`mapChunkOverviewMinZoom`).
pub const OVERVIEW_ZOOM: f32 = 0.9;

/// Niveau de zoom au sens du JS : 1 = une tuile fait 32 pixels.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct Zoom(pub f32);

impl Default for Zoom {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Zoom {
    pub fn is_overview(self) -> bool {
        self.0 < OVERVIEW_ZOOM
    }
}

/// Convertit une tuile (Y vers le bas) en position monde Bevy (Y vers le haut).
pub fn tile_to_world(x: f32, y: f32) -> Vec2 {
    Vec2::new(x * TILE_SIZE, -y * TILE_SIZE)
}

/// Inverse de [`tile_to_world`], en tuiles fractionnaires.
pub fn world_to_tile(world: Vec2) -> Vec2 {
    Vec2::new(world.x / TILE_SIZE, -world.y / TILE_SIZE)
}

/// Systèmes de la caméra ; le rendu de la carte passe après.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CameraSet;

#[derive(Debug, Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Zoom>()
            .add_systems(OnEnter(AppState::InGame), reset_camera)
            .add_systems(OnExit(AppState::InGame), reset_camera)
            .add_systems(
                Update,
                (keyboard_pan, drag_pan, wheel_zoom, apply_zoom)
                    .chain()
                    .in_set(CameraSet)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

fn reset_camera(mut zoom: ResMut<Zoom>, mut camera: Query<&mut Transform, With<Camera2d>>) {
    zoom.0 = 1.0;
    for mut t in &mut camera {
        t.translation = Vec3::ZERO;
    }
}

fn keyboard_pan(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    zoom: Res<Zoom>,
    mut camera: Single<&mut Transform, With<Camera2d>>,
) {
    let mut dir = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        dir.y += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        dir.y -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        dir.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        dir.x += 1.0;
    }
    if dir != Vec2::ZERO {
        // Vitesse constante à l'écran, quel que soit le zoom.
        let speed = 800.0 / zoom.0;
        camera.translation += (dir.normalize() * speed * time.delta_secs()).extend(0.0);
    }
}

fn drag_pan(
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    zoom: Res<Zoom>,
    mut camera: Single<&mut Transform, With<Camera2d>>,
) {
    if buttons.any_pressed([MouseButton::Middle, MouseButton::Right]) && motion.delta != Vec2::ZERO
    {
        let delta = motion.delta / zoom.0;
        camera.translation.x -= delta.x;
        camera.translation.y += delta.y;
    }
}

fn wheel_zoom(
    mut wheel: MessageReader<MouseWheel>,
    mut zoom: ResMut<Zoom>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform, &mut Transform), With<Camera2d>>,
) {
    let steps: f32 = wheel
        .read()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y / 100.0,
        })
        .sum();
    if steps == 0.0 {
        return;
    }
    let old = zoom.0;
    let new = (old * 1.2_f32.powf(steps)).clamp(MIN_ZOOM, MAX_ZOOM);
    if new == old {
        return;
    }
    zoom.0 = new;

    // Garde le point sous le curseur fixe à l'écran.
    let (camera, global, mut transform) = camera.into_inner();
    if let Some(cursor) = window.cursor_position()
        && let Ok(world) = camera.viewport_to_world_2d(global, cursor)
    {
        let center = transform.translation.truncate();
        let new_center = world + (center - world) * (old / new);
        transform.translation = new_center.extend(transform.translation.z);
    }
}

fn apply_zoom(zoom: Res<Zoom>, mut projection: Single<&mut Projection, With<Camera2d>>) {
    if let Projection::Orthographic(ortho) = projection.as_mut()
        && ortho.scale != 1.0 / zoom.0
    {
        ortho.scale = 1.0 / zoom.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_world_roundtrip() {
        let w = tile_to_world(3.0, -2.0);
        assert_eq!(w, Vec2::new(96.0, 64.0));
        assert_eq!(world_to_tile(w), Vec2::new(3.0, -2.0));
    }
}
