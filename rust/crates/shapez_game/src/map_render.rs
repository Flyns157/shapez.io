//! Rendu de la carte par chunk : fond, ressources et vue d'ensemble au dézoom
//! (port de `systems/map_resources.js` et `map_chunk_view.js`).

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use shapez_core::Color as ShapeColor;
use shapez_sim::map::{CHUNK_SIZE, ChunkPos, ChunkResources, MapResource};

use crate::{
    AppState,
    camera::{TILE_SIZE, Zoom, world_to_tile},
    shape_textures::ShapeTextures,
    sim::Sim,
};

/// Fond de la carte (`THEME.map.background`, thème sombre).
pub const MAP_BACKGROUND: Color = Color::srgb_u8(0x3e, 0x3f, 0x47);
const CHUNK_WORLD_SIZE: f32 = CHUNK_SIZE as f32 * TILE_SIZE;
/// Diamètre d'un item posé sur la carte (`defaultItemDiameter`).
const ITEM_DIAMETER: f32 = 20.0;
/// Chunks gardés en plus autour de l'écran, pour éviter de les recréer sans cesse.
const MARGIN_CHUNKS: i32 = 1;

#[derive(Resource, Debug, Default)]
struct VisibleChunks(HashMap<ChunkPos, Entity>);

/// Entité d'un chunk ; porte les deux images (normale et vue d'ensemble).
#[derive(Component, Debug)]
struct ChunkView {
    background: Handle<Image>,
    overview: Handle<Image>,
}

/// Icône d'une ressource sur une tuile (vue normale).
#[derive(Component, Debug)]
struct ResourceIcon;

/// Icône d'un gisement de formes (vue d'ensemble).
#[derive(Component, Debug)]
struct PatchIcon;

#[derive(Debug, Default)]
pub struct MapRenderPlugin;

impl Plugin for MapRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VisibleChunks>()
            .insert_resource(ClearColor(MAP_BACKGROUND))
            .add_systems(OnExit(AppState::InGame), despawn_all_chunks)
            .add_systems(
                Update,
                (sync_visible_chunks, apply_overview, draw_grid)
                    .chain()
                    .after(crate::camera::CameraSet)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

/// Couleur de fond d'une ressource (`THEME.map.resources`, thème sombre).
fn resource_color(resource: MapResource) -> [u8; 3] {
    match resource {
        MapResource::Shape(_) => [0x5d, 0x5f, 0x6a],
        MapResource::Color(ShapeColor::Red) => [0x85, 0x4f, 0x56],
        MapResource::Color(ShapeColor::Green) => [0x66, 0x79, 0x64],
        MapResource::Color(_) => [0x5e, 0x7c, 0xa4],
    }
}

/// Image 16 × 16 (un pixel par tuile) : ressources à 50 % d'opacité, ou opaques sur
/// le fond `chunkOverview.empty` pour la vue d'ensemble.
fn chunk_image(chunk: &ChunkResources, overview: bool) -> Image {
    let n = CHUNK_SIZE as usize;
    let mut data = Vec::with_capacity(n * n * 4);
    for y in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let px = match (chunk.get(x, y), overview) {
                (Some(r), true) => {
                    let [r, g, b] = resource_color(r);
                    [r, g, b, 0xff]
                }
                (Some(r), false) => {
                    let [r, g, b] = resource_color(r);
                    [r, g, b, 0x80]
                }
                (None, true) => [0x44, 0x48, 0x56, 0xff],
                (None, false) => [0, 0, 0, 0],
            };
            data.extend_from_slice(&px);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: n as u32,
            height: n as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Chunks qui recouvrent l'écran (plus une marge).
fn visible_range(camera_pos: Vec2, half_extent: Vec2) -> (ChunkPos, ChunkPos) {
    let min = world_to_tile(camera_pos + Vec2::new(-half_extent.x, half_extent.y));
    let max = world_to_tile(camera_pos + Vec2::new(half_extent.x, -half_extent.y));
    let to_chunk = |t: f32| (t / CHUNK_SIZE as f32).floor() as i32;
    (
        ChunkPos::new(
            to_chunk(min.x) - MARGIN_CHUNKS,
            to_chunk(min.y) - MARGIN_CHUNKS,
        ),
        ChunkPos::new(
            to_chunk(max.x) + MARGIN_CHUNKS,
            to_chunk(max.y) + MARGIN_CHUNKS,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn sync_visible_chunks(
    mut commands: Commands,
    mut visible: ResMut<VisibleChunks>,
    mut sim: ResMut<Sim>,
    mut images: ResMut<Assets<Image>>,
    mut textures: ResMut<ShapeTextures>,
    zoom: Res<Zoom>,
    window: Single<&Window>,
    camera: Single<&Transform, With<Camera2d>>,
) {
    let half = Vec2::new(window.width(), window.height()) / (2.0 * zoom.0);
    let (min, max) = visible_range(camera.translation.truncate(), half);

    visible.0.retain(|pos, entity| {
        let keep = (min.x..=max.x).contains(&pos.x) && (min.y..=max.y).contains(&pos.y);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });

    let overview = zoom.is_overview();
    for cx in min.x..=max.x {
        for cy in min.y..=max.y {
            let pos = ChunkPos::new(cx, cy);
            if visible.0.contains_key(&pos) {
                continue;
            }
            let chunk = sim.map.chunk(pos).clone();
            let background = images.add(chunk_image(&chunk, false));
            let overview_image = images.add(chunk_image(&chunk, true));
            let origin = pos.origin();
            let center = Vec2::new(
                (origin.x as f32 + CHUNK_SIZE as f32 / 2.0) * TILE_SIZE,
                -(origin.y as f32 + CHUNK_SIZE as f32 / 2.0) * TILE_SIZE,
            );

            let mut entity = commands.spawn((
                ChunkView {
                    background: background.clone(),
                    overview: overview_image.clone(),
                },
                Sprite {
                    image: if overview { overview_image } else { background },
                    custom_size: Some(Vec2::splat(CHUNK_WORLD_SIZE)),
                    ..default()
                },
                Transform::from_translation(center.extend(0.0)),
            ));
            if !chunk.is_empty() {
                entity.with_children(|parent| {
                    spawn_resource_icons(parent, &chunk, &mut textures, &mut images, overview);
                });
            }
            visible.0.insert(pos, entity.id());
        }
    }
}

fn spawn_resource_icons(
    parent: &mut ChildSpawnerCommands,
    chunk: &ChunkResources,
    textures: &mut ShapeTextures,
    images: &mut Assets<Image>,
    overview: bool,
) {
    let half = CHUNK_SIZE as f32 / 2.0;
    // Position locale au chunk (centre du chunk = origine).
    let local = |x: f32, y: f32| Vec3::new((x - half) * TILE_SIZE, -(y - half) * TILE_SIZE, 1.0);
    let normal = if overview {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    let over = if overview {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            let Some(resource) = chunk.get(x, y) else {
                continue;
            };
            parent.spawn((
                ResourceIcon,
                Sprite {
                    image: textures.resource(resource, images),
                    custom_size: Some(Vec2::splat(ITEM_DIAMETER)),
                    color: Color::WHITE.with_alpha(0.5),
                    ..default()
                },
                Transform::from_translation(local(x as f32 + 0.5, y as f32 + 0.5)),
                normal,
            ));
        }
    }
    for patch in chunk.patches() {
        if let MapResource::Shape(_) = patch.resource {
            parent.spawn((
                PatchIcon,
                Sprite {
                    image: textures.resource(patch.resource, images),
                    custom_size: Some(Vec2::splat(ITEM_DIAMETER * 4.0)),
                    ..default()
                },
                Transform::from_translation(local(patch.center.0 as f32, patch.center.1 as f32)),
                over,
            ));
        }
    }
}

/// Bascule entre vue normale et vue d'ensemble quand le zoom franchit le seuil.
fn apply_overview(
    zoom: Res<Zoom>,
    mut chunks: Query<(&ChunkView, &mut Sprite)>,
    mut icons: Query<&mut Visibility, (With<ResourceIcon>, Without<PatchIcon>)>,
    mut patches: Query<&mut Visibility, (With<PatchIcon>, Without<ResourceIcon>)>,
    mut was_overview: Local<Option<bool>>,
) {
    let overview = zoom.is_overview();
    if *was_overview == Some(overview) {
        return;
    }
    *was_overview = Some(overview);
    for (view, mut sprite) in &mut chunks {
        sprite.image = if overview {
            view.overview.clone()
        } else {
            view.background.clone()
        };
    }
    let (normal, over) = if overview {
        (Visibility::Hidden, Visibility::Inherited)
    } else {
        (Visibility::Inherited, Visibility::Hidden)
    };
    icons.iter_mut().for_each(|mut v| *v = normal);
    patches.iter_mut().for_each(|mut v| *v = over);
}

/// Grille des tuiles (`THEME.map.gridRegular`), seulement en vue normale.
fn draw_grid(
    mut gizmos: Gizmos,
    zoom: Res<Zoom>,
    window: Single<&Window>,
    camera: Single<&Transform, With<Camera2d>>,
) {
    if zoom.is_overview() {
        return;
    }
    let half = Vec2::new(window.width(), window.height()) / (2.0 * zoom.0);
    let center = camera.translation.truncate();
    let cells = (half * 2.0 / TILE_SIZE).ceil() + Vec2::splat(2.0);
    // Aligne la grille sur les tuiles.
    let snapped = (center / TILE_SIZE).round() * TILE_SIZE;
    gizmos.grid_2d(
        Isometry2d::from_translation(snapped),
        UVec2::new(cells.x as u32, cells.y as u32),
        Vec2::splat(TILE_SIZE),
        Color::srgba(1.0, 1.0, 1.0, 0.025),
    );
}

fn despawn_all_chunks(mut commands: Commands, mut visible: ResMut<VisibleChunks>) {
    for (_, entity) in visible.0.drain() {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_range_covers_screen() {
        // Écran de 1280 × 720 centré sur l'origine, zoom 1 : x de -20 à 20 tuiles.
        let (min, max) = visible_range(Vec2::ZERO, Vec2::new(640.0, 360.0));
        assert_eq!((min.x, max.x), (-2 - MARGIN_CHUNKS, 1 + MARGIN_CHUNKS));
        assert_eq!((min.y, max.y), (-1 - MARGIN_CHUNKS, MARGIN_CHUNKS));
    }
}
