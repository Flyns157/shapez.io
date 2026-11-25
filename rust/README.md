# shapez-rs

Réécriture de shapez.io en Rust avec [Bevy](https://bevy.org) 0.20. Le plan complet est
dans [`../docs/plan-rust-bevy.md`](../docs/plan-rust-bevy.md).

## Organisation

| Crate | Rôle | Dépend de Bevy |
|---|---|---|
| `crates/shapez_core` | Grille, formes (codes courts, opérations), couleurs, items, registre | non |
| `crates/shapez_render_shapes` | Rastérisation des formes en RGBA (tiny-skia) | non |
| `crates/shapez_sim` | Simulation déterministe à 60 ticks/s, en entiers | non |
| `crates/shapez_game` | Binaire Bevy : rendu, entrées, UI | oui |
| `xtask` | Tâches de développement | non |

Les autres crates du plan (`shapez_data`, `shapez_save`)
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

## Oracle JS

Le code JS d'origine sert de spécification. `tools/js-oracle/` l'exécute dans Node
(sans webpack) pour produire des vecteurs de test versionnés ; la CI n'a pas besoin de Node.

```sh
node tools/js-oracle/shapes.mjs > crates/shapez_core/tests/golden/shapes.json   # 2 000 cas
node tools/js-oracle/shapes.mjs 10000 > /tmp/shapes.json
SHAPEZ_GOLDEN_SHAPES=/tmp/shapes.json cargo test -p shapez_core --test golden_shapes
```

## Licence

GPL-3.0-or-later, comme le jeu d'origine. Voir [`NOTICE`](NOTICE) et [`../LICENSE`](../LICENSE).
