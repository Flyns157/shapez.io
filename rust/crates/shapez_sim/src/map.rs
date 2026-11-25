//! Carte : génération des ressources par chunk, port de `src/js/game/map_chunk.js`.
//!
//! La génération est une fonction pure de `(graine, chunk)`. Elle reproduit le JS
//! opération par opération (Alea, `Math.round`, calculs en `f64`), si bien qu'une même
//! graine donne **la même carte** que le jeu d'origine. Les flottants ne servent qu'ici,
//! à la création du terrain ; la logique par tick reste en entiers.

use std::collections::HashMap;

use shapez_core::{Alea, Color, ShapeDef, SubShape, TilePos, rng::js_round};

/// Côté d'un chunk, en tuiles (`globalConfig.mapChunkSize`).
pub const CHUNK_SIZE: i32 = 16;
const SIZE: f64 = CHUNK_SIZE as f64;
/// Le JS teste `y <= mapChunkSize` au lieu de `<` : une ligne fantôme y == 16 peut être
/// remplie. Elle n'est jamais visible, mais elle compte dans la forme des gisements.
const ROWS_WITH_GHOST: usize = CHUNK_SIZE as usize + 1;

/// Ressource extractible posée sur une tuile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapResource {
    Shape(ShapeDef),
    Color(Color),
}

/// Gisement, tel que stocké par le JS pour la vue d'ensemble (`patches`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    /// Position moyenne des tuiles posées, en coordonnées locales au chunk.
    pub center: (f64, f64),
    pub resource: MapResource,
    pub size: u32,
}

/// Coordonnées d'un chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Chunk qui contient la tuile.
    pub const fn of_tile(tile: TilePos) -> Self {
        Self::new(tile.x.div_euclid(CHUNK_SIZE), tile.y.div_euclid(CHUNK_SIZE))
    }

    /// Tuile en haut à gauche du chunk.
    pub const fn origin(self) -> TilePos {
        TilePos::new(self.x * CHUNK_SIZE, self.y * CHUNK_SIZE)
    }
}

/// Couche basse d'un chunk : les ressources de chaque tuile.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkResources {
    /// Index dans `palette` + 1, 0 = vide. Indexé `[x][y]` comme le JS.
    tiles: [[u8; ROWS_WITH_GHOST]; CHUNK_SIZE as usize],
    palette: Vec<MapResource>,
    patches: Vec<Patch>,
}

impl ChunkResources {
    fn empty() -> Self {
        Self {
            tiles: [[0; ROWS_WITH_GHOST]; CHUNK_SIZE as usize],
            palette: Vec::new(),
            patches: Vec::new(),
        }
    }

    /// Ressource à une position locale (0..16). Hors chunk : `None`.
    pub fn get(&self, x: i32, y: i32) -> Option<MapResource> {
        if !(0..CHUNK_SIZE).contains(&x) || !(0..CHUNK_SIZE).contains(&y) {
            return None;
        }
        self.get_raw(x as usize, y as usize)
    }

    fn get_raw(&self, x: usize, y: usize) -> Option<MapResource> {
        match self.tiles[x][y] {
            0 => None,
            i => Some(self.palette[i as usize - 1]),
        }
    }

    /// Y compris la ligne fantôme y == 16 (pour comparer au JS).
    pub fn get_with_ghost_row(&self, x: usize, y: usize) -> Option<MapResource> {
        self.get_raw(x, y)
    }

    pub fn patches(&self) -> &[Patch] {
        &self.patches
    }

    pub fn is_empty(&self) -> bool {
        self.palette.is_empty()
    }

    fn palette_index(&mut self, resource: MapResource) -> u8 {
        let idx = match self.palette.iter().position(|r| *r == resource) {
            Some(i) => i,
            None => {
                self.palette.push(resource);
                self.palette.len() - 1
            }
        };
        u8::try_from(idx + 1).expect("moins de 255 ressources par chunk")
    }

    /// `internalGeneratePatch`.
    fn generate_patch(
        &mut self,
        rng: &mut Alea,
        patch_size: u32,
        resource: MapResource,
        override_pos: (Option<i64>, Option<i64>),
    ) {
        let size = f64::from(patch_size);
        let border = (size / 2.0 + 3.0).ceil();
        let mut patch_x = rng.next_int_range(border, SIZE - border - 1.0);
        let mut patch_y = rng.next_int_range(border, SIZE - border - 1.0);
        if let Some(x) = override_pos.0 {
            patch_x = x;
        }
        if let Some(y) = override_pos.1 {
            patch_y = y;
        }

        let index = self.palette_index(resource);
        let (mut sum_x, mut sum_y, mut drawn) = (0.0, 0.0, 0u32);
        let num_circles = size;

        for i in 0..=patch_size {
            let i = f64::from(i);
            let radius = (1.0 + i).min(size);
            let radius_sq = radius * radius;
            let offset_radius = (num_circles - i) / 2.0 + 2.0;
            let scale_x = rng.next_range(0.9, 1.1);
            let scale_y = rng.next_range(0.9, 1.1);
            let circle_x =
                patch_x as f64 + rng.next_int_range(-offset_radius, offset_radius) as f64;
            let circle_y =
                patch_y as f64 + rng.next_int_range(-offset_radius, offset_radius) as f64;

            let mut dx = -radius * scale_x - 2.0;
            while dx <= radius * scale_x + 2.0 {
                let mut dy = -radius * scale_y - 2.0;
                while dy <= radius * scale_y + 2.0 {
                    let x = js_round(circle_x + dx);
                    let y = js_round(circle_y + dy);
                    if (0.0..SIZE).contains(&x) && (0.0..=SIZE).contains(&y) {
                        let (odx, ody) = (dx / scale_x, dy / scale_y);
                        let tile = &mut self.tiles[x as usize][y as usize];
                        if odx * odx + ody * ody <= radius_sq && *tile == 0 {
                            *tile = index;
                            drawn += 1;
                            sum_x += x;
                            sum_y += y;
                        }
                    }
                    dy += 1.0;
                }
                dx += 1.0;
            }
        }

        let n = f64::from(drawn);
        self.patches.push(Patch {
            center: (sum_x / n, sum_y / n),
            resource,
            size: patch_size,
        });
    }
}

fn clamp(v: f64, min: f64, max: f64) -> f64 {
    min.max(max.min(v))
}

fn shape(key: &str) -> MapResource {
    MapResource::Shape(key.parse().expect("forme prédéfinie valide"))
}

/// Génère la couche basse du chunk (`generateLowerLayer`).
pub fn generate_chunk(seed: u32, pos: ChunkPos) -> ChunkResources {
    let mut chunk = ChunkResources::empty();
    let mut rng = Alea::new(&format!("{}|{}|{}", pos.x, pos.y, seed));

    if generate_predefined(&mut chunk, &mut rng, pos) {
        return chunk;
    }

    let (cx, cy) = (f64::from(pos.x) + 0.5, f64::from(pos.y) + 0.5);
    let distance = js_round(cx.hypot(cy));

    let color_chance = 0.9 - clamp(distance / 25.0, 0.0, 1.0) * 0.5;
    if rng.next_f64() < color_chance / 4.0 {
        let size = patch_size(distance);
        let mut colors = vec![Color::Red, Color::Green];
        if distance > 2.0 {
            colors.push(Color::Blue);
        }
        let color = *rng.choice(&colors);
        chunk.generate_patch(&mut rng, size, MapResource::Color(color), (None, None));
    }

    let shape_chance = 0.9 - clamp(distance / 25.0, 0.0, 1.0) * 0.5;
    if rng.next_f64() < shape_chance / 4.0 {
        let size = patch_size(distance);
        let def = random_patch_shape(&mut rng, distance);
        chunk.generate_patch(&mut rng, size, MapResource::Shape(def), (None, None));
    }
    chunk
}

fn patch_size(distance: f64) -> u32 {
    js_round(1.0 + clamp(distance / 8.0, 0.0, 4.0)).max(2.0) as u32
}

/// `generatePredefined` : chunks fixes autour de l'origine.
fn generate_predefined(chunk: &mut ChunkResources, rng: &mut Alea, pos: ChunkPos) -> bool {
    let s = i64::from(CHUNK_SIZE);
    let (resource, at) = match (pos.x, pos.y) {
        (0, 0) => (MapResource::Color(Color::Red), (Some(7), Some(7))),
        (-1, 0) => (shape("CuCuCuCu"), (Some(s - 9), Some(7))),
        (0, -1) => (shape("RuRuRuRu"), (Some(5), Some(s - 7))),
        (-1, -1) => (MapResource::Color(Color::Green), (None, None)),
        (5, -2) => (shape("SuSuSuSu"), (Some(5), Some(s - 7))),
        _ => return false,
    };
    chunk.generate_patch(rng, 2, resource, at);
    true
}

/// `internalGenerateShapePatch` : forme non colorée d'un gisement.
fn random_patch_shape(rng: &mut Alea, distance: f64) -> ShapeDef {
    // Ordre d'insertion des clés de l'objet `weights` du JS.
    let mut weights = [
        (SubShape::Rect, 100.0),
        (
            SubShape::Circle,
            js_round(50.0 + clamp(distance * 2.0, 0.0, 50.0)),
        ),
        (SubShape::Star, js_round(20.0 + clamp(distance, 0.0, 30.0))),
        (
            SubShape::Windmill,
            js_round(6.0 + clamp(distance / 2.0, 0.0, 20.0)),
        ),
    ];
    if distance < 7.0 {
        weights[2].1 = 0.0;
        weights[3].1 = 0.0;
    }

    let mut subs = if distance < 10.0 {
        let s = random_sub_shape(rng, &weights);
        [s; 4]
    } else if distance < 15.0 {
        let a = random_sub_shape(rng, &weights);
        let b = random_sub_shape(rng, &weights);
        [a, a, b, b]
    } else {
        std::array::from_fn(|_| random_sub_shape(rng, &weights))
    };

    // Jamais de moulin complet.
    if subs.iter().filter(|s| **s == SubShape::Windmill).count() > 1 {
        subs[0] = SubShape::Rect;
        subs[1] = SubShape::Rect;
    }
    ShapeDef::simple(subs.map(Some), Color::Uncolored)
}

/// `internalGenerateRandomSubShape`.
fn random_sub_shape(rng: &mut Alea, weights: &[(SubShape, f64); 4]) -> SubShape {
    let sum: f64 = weights.iter().map(|w| w.1).sum();
    let chosen = rng.next_int_range(0.0, sum - 1.0) as f64;
    let mut accumulated = 0.0;
    for &(sub, weight) in weights {
        if accumulated + weight > chosen {
            return sub;
        }
        accumulated += weight;
    }
    SubShape::Circle
}

/// Carte complète : chunks générés à la demande, mis en cache.
///
/// Le `HashMap` ne sert qu'aux recherches ; la logique n'itère jamais dessus.
#[derive(Debug, Clone)]
pub struct Map {
    seed: u32,
    chunks: HashMap<ChunkPos, ChunkResources>,
}

impl Map {
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            chunks: HashMap::new(),
        }
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    pub fn chunk(&mut self, pos: ChunkPos) -> &ChunkResources {
        let seed = self.seed;
        self.chunks
            .entry(pos)
            .or_insert_with(|| generate_chunk(seed, pos))
    }

    /// Ressource sous une tuile (`getLowerLayerContentXY`).
    pub fn resource_at(&mut self, tile: TilePos) -> Option<MapResource> {
        let pos = ChunkPos::of_tile(tile);
        let local = tile - pos.origin();
        self.chunk(pos).get(local.x, local.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_area_is_predefined() {
        let mut map = Map::new(42);
        assert_eq!(
            map.resource_at(TilePos::new(7, 7)),
            Some(MapResource::Color(Color::Red))
        );
        assert_eq!(
            map.resource_at(TilePos::new(-9, 7)),
            Some(shape("CuCuCuCu"))
        );
        assert_eq!(
            map.resource_at(TilePos::new(5, -7)),
            Some(shape("RuRuRuRu"))
        );
    }

    #[test]
    fn chunk_of_negative_tiles() {
        assert_eq!(ChunkPos::of_tile(TilePos::new(-1, 0)), ChunkPos::new(-1, 0));
        assert_eq!(
            ChunkPos::of_tile(TilePos::new(-16, 15)),
            ChunkPos::new(-1, 0)
        );
        assert_eq!(
            ChunkPos::of_tile(TilePos::new(-17, 16)),
            ChunkPos::new(-2, 1)
        );
    }

    #[test]
    fn generation_is_pure() {
        let p = ChunkPos::new(13, -8);
        assert_eq!(generate_chunk(7, p), generate_chunk(7, p));
    }
}
