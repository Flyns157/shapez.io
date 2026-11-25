//! Galerie des formes : vérifie visuellement la rastérisation et les opérations (M1).

use bevy::prelude::*;
use shapez_core::{Color as ShapeColor, ShapeDef};

use crate::{AppState, shape_textures::ShapeTextures};

/// Objectifs des niveaux du jeu d'origine (`modes/levels.js`), dans l'ordre.
const LEVEL_SHAPES: &[&str] = &[
    "CuCuCuCu",
    "----CuCu",
    "RuRuRuRu",
    "RuRu----",
    "Cu----Cu",
    "Cu------",
    "CrCrCrCr",
    "RbRb----",
    "CpCpCpCp",
    "ScScScSc",
    "CgScScCg",
    "CbCbCbRb:CwCwCwCw",
    "RpRpRpRp:CwCwCwCw",
    "--Cg----:--Cr----",
    "SrSrSrSr:CyCyCyCy",
    "SrSrSrSr:CyCyCyCy:SwSwSwSw",
    "CbRbRbCb:CwCwCwCw:WbWbWbWb",
    "Sg----Sg:CgCgCgCg:--CyCy--",
    "CpRpCp--:SwSwSwSw",
    "RuCw--Cw:----Ru--",
    "CrCwCrCw:CwCrCwCr:CrCwCrCw:CwCrCwCr",
    "Cg----Cr:Cw----Cw:Sy------:Cy----Cy",
    "CbCuCbCu:Sr------:--CrSrCr:CwCwCwCw",
];

#[derive(Debug, Default)]
pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::ShapeGallery), spawn_gallery)
            .add_systems(
                Update,
                back_to_menu.run_if(in_state(AppState::ShapeGallery)),
            );
    }
}

/// Formes affichées : objectifs des niveaux, puis chaque opération appliquée à une forme.
fn gallery_shapes() -> Vec<(String, ShapeDef)> {
    let mut out: Vec<(String, ShapeDef)> = LEVEL_SHAPES
        .iter()
        .map(|k| (k.to_string(), k.parse().expect("forme valide")))
        .collect();
    let base: ShapeDef = "RuCw--Cw:----Ru--".parse().expect("forme valide");
    let [west, east] = base.cut_half();
    let ops = [
        ("decoupe G", west),
        ("decoupe D", east),
        ("rotation", base.rotate_cw()),
        ("rotation inv.", base.rotate_ccw()),
        ("demi-tour", base.rotate_180()),
        (
            "empilement",
            west.stack(&"SySySySy".parse().expect("forme valide")),
        ),
        ("peinture", base.paint(ShapeColor::Purple)),
    ];
    out.extend(
        ops.into_iter()
            .map(|(name, def)| (format!("{name} : {def}"), def)),
    );
    out
}

fn spawn_gallery(
    mut commands: Commands,
    mut textures: ResMut<ShapeTextures>,
    mut images: ResMut<Assets<Image>>,
) {
    let cells: Vec<_> = gallery_shapes()
        .into_iter()
        .map(|(label, def)| (label, textures.get_or_render(def, &mut images)))
        .collect();

    commands
        .spawn((
            DespawnOnExit(AppState::ShapeGallery),
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(16)),
                row_gap: px(12),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(Text::new("Galerie des formes (Echap = menu)"));
            root.spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(8),
                row_gap: px(8),
                ..default()
            })
            .with_children(|grid| {
                for (label, image) in cells {
                    grid.spawn(Node {
                        width: px(150),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|cell| {
                        cell.spawn((
                            ImageNode::new(image),
                            Node {
                                width: px(96),
                                height: px(96),
                                ..default()
                            },
                        ));
                        cell.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: FontSize::Px(10.0),
                                ..default()
                            },
                        ));
                    });
                }
            });
        });
}

fn back_to_menu(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(AppState::MainMenu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gallery_shapes_are_valid() {
        let shapes = gallery_shapes();
        assert_eq!(shapes.len(), LEVEL_SHAPES.len() + 7);
        assert!(shapes.iter().all(|(_, d)| !d.is_empty()));
    }
}
