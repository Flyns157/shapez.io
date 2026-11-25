//! Textures des formes, rastérisées à la demande par `shapez_render_shapes`.
//!
//! Version simple du `ShapeAtlasPlugin` du plan (§6.3) : une image par forme, en cache.
//! L'atlas dynamique avec LRU arrivera avec le rendu des items (M4).

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use shapez_core::ShapeDef;
use shapez_render_shapes::{ShapeTheme, render_rgba};

/// Taille de rastérisation, en pixels.
pub const SHAPE_TEXTURE_SIZE: u32 = 128;

#[derive(Resource, Debug, Default)]
pub struct ShapeTextures {
    cache: HashMap<ShapeDef, Handle<Image>>,
}

impl ShapeTextures {
    /// Renvoie l'image de la forme, en la rastérisant au premier appel.
    pub fn get_or_render(&mut self, def: ShapeDef, images: &mut Assets<Image>) -> Handle<Image> {
        self.cache
            .entry(def)
            .or_insert_with(|| {
                let size = SHAPE_TEXTURE_SIZE;
                let data = render_rgba(&def, size, &ShapeTheme::DARK);
                images.add(Image::new(
                    Extent3d {
                        width: size,
                        height: size,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    data,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::RENDER_WORLD,
                ))
            })
            .clone()
    }
}

#[derive(Debug, Default)]
pub struct ShapeTexturesPlugin;

impl Plugin for ShapeTexturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShapeTextures>();
    }
}
