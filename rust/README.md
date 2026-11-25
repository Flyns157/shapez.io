# shapez-rs

Réécriture de shapez.io en Rust avec [Bevy](https://bevy.org) 0.20. Le plan complet est
dans [`../docs/plan-rust-bevy.md`](../docs/plan-rust-bevy.md).

## Organisation

| Crate | Rôle | Dépend de Bevy |
|---|---|---|
| `crates/shapez_core` | Grille, directions, rotations (puis formes, couleurs, items) | non |
| `crates/shapez_sim` | Simulation déterministe à 60 ticks/s, en entiers | non |
| `crates/shapez_game` | Binaire Bevy : rendu, entrées, UI | oui |
| `xtask` | Tâches de développement | non |

Les autres crates du plan (`shapez_data`, `shapez_save`, `shapez_render_shapes`)
seront ajoutées avec les jalons qui en ont besoin.

## Démarrer

Prérequis Linux : `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`.

```sh
cd rust
cargo xtask import-assets        # copie sprites, sons, polices et traductions dans assets/
cargo run                        # ouvre la fenêtre du jeu
cargo run --features dev         # idem, avec lien dynamique et outils de debug
cargo test --workspace
```

`cargo xtask import-assets --music` importe aussi les musiques (~185 Mo).

## Licence

GPL-3.0-or-later, comme le jeu d'origine. Voir [`NOTICE`](NOTICE) et [`../LICENSE`](../LICENSE).
