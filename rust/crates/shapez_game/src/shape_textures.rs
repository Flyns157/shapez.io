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
use shapez_core::{Color as ShapeColor, ShapeDef};
use shapez_render_shapes::{ShapeTheme, render_color_rgba, render_rgba};
use shapez_sim::map::MapResource;

/// Taille de rastérisation, en pixels.
pub const SHAPE_TEXTURE_SIZE: u32 = 128;

#[derive(Resource, Debug, Default)]
pub struct ShapeTextures {
    shapes: HashMap<ShapeDef, Handle<Image>>,
    colors: HashMap<ShapeColor, Handle<Image>>,
}

impl ShapeTextures {
    /// Renvoie l'image de la forme, en la rastérisant au premier appel.
    pub fn get_or_render(&mut self, def: ShapeDef, images: &mut Assets<Image>) -> Handle<Image> {
        self.shapes
            .entry(def)
            .or_insert_with(|| {
                images.add(rgba_image(render_rgba(
                    &def,
                    SHAPE_TEXTURE_SIZE,
                    &ShapeTheme::DARK,
                )))
            })
            .clone()
    }

    /// Image d'un item couleur.
    pub fn get_or_render_color(
        &mut self,
        color: ShapeColor,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        self.colors
            .entry(color)
            .or_insert_with(|| {
                images.add(rgba_image(render_color_rgba(
                    color,
                    SHAPE_TEXTURE_SIZE,
                    &ShapeTheme::DARK,
                )))
            })
            .clone()
    }

    /// Image d'une ressource de la carte.
    pub fn resource(&mut self, resource: MapResource, images: &mut Assets<Image>) -> Handle<Image> {
        match resource {
            MapResource::Shape(def) => self.get_or_render(def, images),
            MapResource::Color(color) => self.get_or_render_color(color, images),
        }
    }
}

fn rgba_image(data: Vec<u8>) -> Image {
    let size = SHAPE_TEXTURE_SIZE;
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[derive(Debug, Default)]
pub struct ShapeTexturesPlugin;

impl Plugin for ShapeTexturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShapeTextures>();
    }
}
