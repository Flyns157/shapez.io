# Plan de réécriture de shapez en Rust avec Bevy

> Document de planification, sans code de jeu. Il s'appuie sur la lecture du code JS
> de ce dépôt (shapez 1.5.6, `src/js/`) et vise **Bevy 0.20** (dernière version stable
> au 25/11/2026, 0.20.1).

---

## 0. Résumé exécutif

1. **Séparer la simulation du moteur.** Toute la logique de jeu (carte, bâtiments,
   convoyeurs, formes, hub, upgrades, wires) vit dans des crates Rust *pures*, sans
   dépendance à Bevy. Bevy ne sert qu'à afficher, à gérer les entrées, l'UI, l'audio et
   le cycle de vie de l'application. On peut ainsi tester et benchmarker la simulation en
   headless, et garantir le déterminisme.
2. **Tick fixe et arithmétique entière.** Le JS fait avancer le monde avec un `deltaSeconds`
   qui dépend du réglage de rafraîchissement (60/120/144 Hz) : son comportement n'est donc pas
   déterministe. En Rust, on simule à **60 ticks/s fixes** dans le schedule `FixedUpdate`,
   avec les positions sur convoyeur en **virgule fixe entière**. On accélère en exécutant
   plusieurs ticks par frame.
3. **Porter fidèlement `BeltPath`.** C'est le cœur des performances du jeu original : les
   convoyeurs contigus sont fusionnés en un chemin unique qui stocke les items par
   *espacement relatif*. On porte cet algorithme tel quel, puis on l'optimise (SoA,
   parallélisme en deux phases).
4. **MVP jouable d'abord.** Le premier jalon jouable couvre les niveaux 1 à 10 environ :
   extracteurs, convoyeurs, découpeur, rotateur, empileur, peintre, mélangeur, équilibreur,
   tunnels et hub. Les wires, les blueprints, le mode puzzle et les mods viennent après.
5. **Licence : GPLv3 obligatoire si l'on réutilise quoi que ce soit.** Le dépôt est sous
   GPLv3. Réutiliser les sprites, les sons, les traductions, les données de niveaux, ou porter
   l'algorithme ligne à ligne fait du projet Rust une œuvre dérivée, qui doit donc être
   publiée sous GPLv3. Il faut aussi éviter le nom « shapez » pour une distribution publique
   (voir §2).
6. **Mémoire : arènes typées dans la simulation.** Bâtiments, `BeltPath` et composants vivent
   dans des arènes à index générationnels (`slotmap`), sans `Rc<RefCell>`. `bumpalo` n'arrive
   pour les données temporaires que si un profil le justifie, et Bevy n'a pas besoin d'arène
   (voir §3.3).

---

## 1. Cartographie du jeu JS existant

### 1.1 Volume et organisation

| Zone | Fichiers | Rôle |
|---|---|---|
| `src/js/core/` | ~45 | Moteur maison : boucle, états, input, sprites/atlas, RNG (Alea), signaux, config globale |
| `src/js/game/` | ~60 + sous-dossiers | Logique de jeu : `root`, `map`, `entity_manager`, `belt_path`, `shape_definition`, `hub_goals`, `camera`, `blueprint` |
| `src/js/game/components/` | 21 | Composants ECS maison (StaticMapEntity, Belt, ItemEjector, ItemAcceptor, ItemProcessor, Miner, Wire...) |
| `src/js/game/systems/` | 23 | Systèmes ECS maison, exécutés dans un **ordre fixe** |
| `src/js/game/buildings/` | 28 | « MetaBuildings » : définition, variantes, déblocage, composants à attacher |
| `src/js/game/hud/parts/` | 48 | UI (HTML/DOM + canvas) : placement, toolbar, shop, stats, menus, tutoriel, puzzle |
| `src/js/game/modes/` | 5 | Mode normal (niveaux et upgrades), puzzle (DLC) |
| `src/js/savegame/` | ~12 + 11 schémas | Sérialisation par schéma, migrations de version 1000 à 1010 |
| `translations/` | 34 YAML | Toutes les chaînes du jeu |
| `res_raw/` | 189 Mo | Sprites sources, sons (sfx en `.wav`, 4 musiques) |

Le jeu JS a déjà une architecture ECS (entités, composants, systèmes). La traduction vers
une architecture ECS en Rust est donc naturelle.

### 1.2 Constantes clés (`core/config.js`)

| Constante | Valeur | Remarque |
|---|---|---|
| `tileSize` | 32 px | Rendu uniquement |
| `mapChunkSize` | 16 tuiles | Agrégats de 4×4 chunks pour la vue d'ensemble |
| `beltSpeedItemsPerSecond` | 2 | × amélioration « belt » |
| `minerSpeedItemsPerSecond` | 0,4 (= 2/5) | × amélioration « miner » |
| `itemSpacingOnBelts` | 0,63 tuile | Distance minimale entre deux items |
| `undergroundBeltMaxTilesByTier` | [5, 9] | Portée des tunnels T1/T2 |
| `buildingSpeeds` | cutter 1/4, rotater 1, painter 1/6, painterDouble 1/8, painterQuad 1/2, mixer 1/5, stacker 1/8 | Multiplicateurs de la vitesse de base |
| `minimumTickRate` / `maximumTickRate` | 25 / 500 | Tick rate dynamique (à **abandonner**) |

### 1.3 Ordre des systèmes (`game_system_manager.js`)

L'ordre compte, car il détermine qui prend un item en premier au cours d'un tick :

```
itemAcceptor → belt → undergroundBelt → miner → storage → itemProcessor → filter
→ itemProducer → itemEjector → mapResources(rendu) → hub → staticMapEntities(rendu)
→ wiredPins → beltUnderlays → constantSignal → lever → wire → logicGate → beltReader
→ display → itemProcessorOverlays → constantProducer → goalAcceptor → zone
```

On garde **le même ordre** pour la partie simulation : les factories des joueurs réagiront
ainsi comme dans l'original.

### 1.4 Systèmes à porter

| Système JS | Fonctionnement | Équivalent Rust |
|---|---|---|
| **Formes** (`shape_definition.js`) | Jusqu'à 4 couches × 4 quadrants (TR, BR, BL, TL), 4 sous-formes (R, C, S, W) et 9 couleurs (u, r, g, b, y, p, c, w, -). Code court du type `RuCw--Cw:----Ru--`. Opérations : couper, rotation CW/CCW/180, empiler, peindre | `shapez_core::shape`, type valeur `ShapeDef` compact, avec les opérations en fonctions pures et un interning via `ShapeId(u32)` |
| **Couleurs** (`colors.js`) | Table de mélange des couleurs | `shapez_core::color`, table `const` |
| **Items** (`items/`) | ShapeItem, ColorItem, BooleanItem | `enum Item { Shape(ShapeId), Color(Color), Bool(bool) }` en `Copy` sur 8 octets |
| **Carte** (`map.js`, `map_chunk.js`) | Chunks 16×16 générés à la demande, patchs de ressources selon la distance à l'origine, chunk (0,0) prédéfini, RNG Alea seedée | `shapez_sim::map`, `HashMap<ChunkPos, Chunk>`, port exact d'Alea pour obtenir les **mêmes cartes** que le JS |
| **Entités statiques** (`static_map_entity.js`) | Origine, rotation, taille, code de bâtiment ; occupation de la grille par couche (normale / wires) | Grille d'occupation par chunk : `[[Option<BuildingId>; 16]; 16]` × 2 couches |
| **Convoyeurs** (`belt_path.js`, 1 661 lignes) | Chemins fusionnés, items stockés en `[distanceToNext, item]`, `spacingToFirstItem`, compression des items bloqués, accepteur final lié | `shapez_sim::belt` (voir §5) |
| **Tunnels** (`underground_belt.js`) | Appariement entrée/sortie dans la portée, file d'items avec temps restant | Composant tunnel et file `VecDeque<(Item, ticks)>` |
| **Éjecteurs / accepteurs** (`item_ejector.js`, `item_acceptor.js`) | Slots positionnés et orientés, progression d'animation, cache de destination | Slots en SoA, cache de destination invalidé par zone modifiée |
| **Processeurs** (`item_processor.js`) | Types : balancer, cutter, cutterQuad, rotater ×3, stacker, trash, mixer, painter ×3, hub, filter, reader, goal ; recharge selon la vitesse | `enum ProcessorKind`, plus une table de recettes pure |
| **Extracteurs** (`miner.js`) | Production périodique, variante « chainable » qui passe l'item à l'extracteur suivant | Système `miner` |
| **Hub et objectifs** (`hub_goals.js`, `modes/levels.js`, `modes/regular.js`) | 26 niveaux puis freeplay (forme générée aléatoirement), upgrades belt/miner/processors/painting par paliers, puis paliers infinis | Données en RON, logique dans `shapez_sim::progress` |
| **Wires** (`wire.js`, `logic_gate.js`...) | Couche séparée débloquée au niveau 20, réseaux recalculés à la modification, valeur unique par réseau avec détection de conflits, portes logiques, lecteurs, afficheurs, processeurs virtuels | `shapez_sim::wires` : union-find et recalcul paresseux |
| **Stockage, filtre, trash, producteurs** | Simples | Systèmes simples |
| **Statistiques** (`production_analytics.js`) | Tranches de 10 s, produit, livré, stocké | Ring buffers par forme |
| **Sauvegarde** (`savegame/`) | JSON compressé par `compressObject` (dictionnaire de clés, entiers en base 190), puis LZ-string, CRC et sel. Migrations 1000 à 1010 | `shapez_save` : serde + postcard + zstd, en-tête versionné, **import optionnel des sauvegardes JS** |
| **Caméra** (`camera.js`, 1 039 lignes) | Pan, zoom 0,1 à 3, inertie, bords, zoom vers le curseur | Plugin caméra maison |
| **Rendu** (`map_chunk_view.js`, `belt_path.draw`) | Canvas 2D, rendu par chunk, mode « overview » sous un zoom de 0,9, formes rastérisées en cache | Sprites et maillages Bevy, rastérisation des formes via tiny-skia (voir §6) |
| **HUD** (48 parties) | DOM et canvas | bevy_egui pour le MVP, puis bevy_ui (voir §7) |
| **Blueprints** (`blueprint.js`) | Copier-coller de zones, coût en formes | `shapez_sim::blueprint` |
| **Mode puzzle** (DLC) | Éditeur et lecture, API en ligne | **Hors périmètre** initial |
| **Mods** (`src/js/mods/`) | API JS d'injection | **Hors périmètre** (voir §11) |

---

## 2. Licence, assets et nom

### 2.1 Ce que dit le dépôt

- `LICENSE` : **GNU GPL v3** pour l'ensemble du dépôt. Aucun fichier de licence séparé
  n'existe pour `res/` ou `res_raw/`.
- Le README indique que les assets sources se trouvent aussi dans le dépôt
  `tobspr-games/shapez.io-artwork`.

### 2.2 Conséquences pratiques

| Ce que l'on réutilise | Conséquence |
|---|---|
| Sprites `res_raw/sprites/**` | Autorisé sous GPLv3. Le jeu Rust doit être GPLv3, avec mention des auteurs |
| Sons `res_raw/sounds/sfx` | Idem |
| Musiques `res_raw/sounds/music` | Présentes dans le dépôt, donc couvertes *a priori* par la GPLv3. **À vérifier** auprès des auteurs (compositeur crédité séparément dans le jeu), ou à remplacer si le projet doit être distribué |
| Traductions `translations/*.yaml` | Réutilisables sous GPLv3, avec mention des contributeurs (`CONTRIBUTORS`) |
| Données de niveaux, upgrades, constantes | Réutilisables sous GPLv3 |
| Algorithmes portés depuis le JS | Œuvre dérivée, donc GPLv3 |

**Recommandation :**

1. Publier le workspace Rust sous **GPL-3.0-or-later** dès le premier commit, avec un
   `NOTICE` qui crédite tobspr Games et les contributeurs d'origine.
2. Le nom et le logo « shapez » sont la marque de tobspr Games. Pour une diffusion
   publique (itch.io, Steam), choisir un autre nom et ne pas réutiliser le logo. Pour un
   projet personnel ou d'apprentissage, cela n'a pas d'incidence.
3. Copier les assets utiles dans `assets/` du workspace avec un script (`xtask import-assets`)
   plutôt qu'à la main. On garde ainsi la traçabilité et on peut les régénérer.
4. Signaler l'existence de **shapez Community Edition** (`tobspr-games/shapez-community-edition`),
   qui reste maintenue : c'est une référence utile pour les corrections de bugs.

---

## 3. Architecture cible

### 3.1 Principe : simulation autoritaire, Bevy en vue

```
┌───────────────────────────── shapez_game (binaire Bevy) ─────────────────────────────┐
│  Input (leafwing) ─▶ Commandes joueur ─▶ ┌──────────────┐ ─▶ Événements sim ─▶ Rendu │
│  UI (egui/bevy_ui)                       │  SimWorld    │     (BuildingAdded,  sync  │
│  Audio, Caméra, Menus                    │ (Resource)   │      ItemDelivered…) ECS   │
│                                          │  tick() x N  │                            │
│  FixedUpdate 60 Hz ─────────────────────▶└──────────────┘                            │
└──────────────────────────────────────────────────────────────────────────────────────┘
        ▲ dépend de                ▲ dépend de             ▲ dépend de
   shapez_render_shapes       shapez_sim  ──▶  shapez_core       shapez_save
   (tiny-skia)                (pur Rust)       (pur Rust)        (serde)
```

**Pourquoi ne pas tout mettre dans l'ECS de Bevy ?**

| Critère | Simulation dans `bevy_ecs` | Simulation dans une crate pure (**retenu**) |
|---|---|---|
| Déterminisme | Ordre d'itération des queries non garanti entre archétypes, `Commands` différés | Ordre total maîtrisé (IDs triés, `Vec`) |
| Performance des convoyeurs | Un item par entité, très coûteux à 100 000 items et plus | Tableaux denses par `BeltPath` |
| Tests | Il faut une `App` Bevy | `cargo test` instantané, sans GPU |
| Montées de version Bevy | Breaking changes tous les ~4 mois sur toute la logique | Seule la couche vue est touchée |
| Sauvegarde | Réflexion ou scènes, verbeux | `serde` direct sur les structures |
| Accès Bevy (rendu, picking) | Natif | Via synchronisation par événements |

L'ECS Bevy reste utilisé pour **tout ce qui est visuel et interactif** : une entité par
bâtiment visible (sprite), une entité par chunk (sol, ressources), le fantôme de placement,
l'UI, la caméra et les sons.

### 3.2 Modèle de données de la simulation

```text
SimWorld
├── config: SimConfig            (vitesses de base, tick rate, constantes)
├── data: GameData (Arc)         (définitions bâtiments, niveaux, upgrades — chargées de RON)
├── shapes: ShapeRegistry        (interning ShapeDef <-> ShapeId, hash = code court)
├── map: Map                     (chunks: ressources + grille d'occupation 2 couches)
├── buildings: SlotMap<BuildingId, Building>
│     Building { kind, variant, origin, rotation, layer, components: index vers les stores }
├── stores (SoA, un par "composant" JS)
│     ejectors, acceptors, processors, miners, undergrounds, storages, filters, wires_pins…
├── belts: BeltNetwork           (SlotMap<PathId, BeltPath>, tile -> PathId)
├── wires: WireNetworks          (réseaux + valeurs, recalcul paresseux)
├── progress: Progress           (niveau, formes stockées dans le hub, upgrades, déblocages)
├── stats: ProductionStats
├── rng / seed
├── tick: u64
└── events: Vec<SimEvent>        (vidé chaque frame par Bevy)
```

### 3.3 Stratégie mémoire : arènes typées

Le JS repose sur des références d'objets croisées (un `BeltPath` pointe vers ses entités,
un éjecteur garde `cachedBeltPath` et `cachedTargetEntity`, etc.). En Rust, les reproduire
avec `Rc<RefCell<…>>` ou des pointeurs serait lent, fragile et non sérialisable. La règle
retenue tient en trois points.

1. **Cœur de simulation (crates sans Bevy) : arènes typées à index générationnels, dès M3/M4.**
   - Une arène par type d'objet : `SlotMap<BuildingId, Building>`, `SlotMap<PathId, BeltPath>`
     et, à l'intérieur de chaque `BeltPath`, des tableaux denses `Vec<Item>` et `Vec<u32>`
     (SoA, §5.4). Les stores de composants (éjecteurs, accepteurs, processeurs…) sont des
     `Vec<T>` indexés par un `u32`, ou des `SecondaryMap<BuildingId, T>`.
   - Toutes les références croisées deviennent des **index** (`BuildingId`, `PathId`,
     `SlotRef { building, slot: u8 }`). La génération embarquée dans l'index détecte les
     références périmées : un cache d'éjecteur qui pointe vers un chemin supprimé est
     simplement invalidé, sans crash ni use-after-free.
   - Bénéfices : pas de `Rc<RefCell>` ni de durées de vie complexes (le borrow checker
     reste simple : on emprunte l'arène, pas l'objet), données contiguës et donc
     amicales pour le cache, ordre d'itération **déterministe** (ordre des slots, jamais
     d'adresses mémoire), et **sérialisation directe** pour la sauvegarde (`slotmap`
     supporte `serde`, les index se sauvegardent tels quels ou sont recompactés au
     chargement).
   - Les items sur convoyeurs ne sont pas des objets individuels alloués : ce sont des
     valeurs `Copy` de 8 octets (`Item`) rangées dans les tableaux du `BeltPath`. Les formes
     sont internées une fois dans le `ShapeRegistry` (`ShapeId(u32)`).
   - Les objets se suppriment via l'arène (libération du slot, génération incrémentée), avec
     une liste de suppression différée vidée en fin de tick pour ne pas invalider des index
     en cours d'itération.

2. **Données temporaires par tick ou par frame : `bumpalo`, uniquement si le profilage le
   justifie.** Exemples : calcul du chemin de pose de convoyeurs lors du glisser, listes de
   candidats pour la résolution des remises (phase B du §5.4), batching du rendu des items,
   recalcul des réseaux de wires. Par défaut, on réutilise des `Vec` conservés d'un tick à
   l'autre (`clear()` sans libérer la capacité), ce qui suffit le plus souvent. `bumpalo`
   (une arène remise à zéro à chaque tick ou frame) n'est introduit qu'**au jalon M10, si
   un profil montre que les allocations temporaires coûtent cher**.

3. **Côté Bevy : pas d'arène.** L'ECS de Bevy range déjà les composants dans des tables
   contiguës par archétype, et ses `Entity` sont elles-mêmes des index générationnels.
   Ajouter une arène par-dessus n'apporterait rien. La correspondance simulation vers rendu
   se fait par une table `HashMap<BuildingId, Entity>` (ou `SecondaryMap`) dans le plugin de
   synchronisation.

### 3.4 Commandes et événements

- **Commandes en entrée** (`enum SimCommand { Place{..}, Remove{..}, Rotate{..},
  PasteBlueprint{..}, BuyUpgrade{..}, SetLever{..}, ... }`) : toute mutation passe par
  une commande. On obtient l'undo/redo, les replays de tests et, plus tard, éventuellement
  le multijoueur en lockstep.
- **Événements en sortie** (`BuildingPlaced`, `BuildingRemoved`, `LevelCompleted`,
  `UpgradeUnlocked`, `ItemDelivered`, etc.) : consommés par Bevy pour le rendu, les sons et
  les notifications.

### 3.5 Plugins Bevy (crate `shapez_game`)

| Plugin | Contenu |
|---|---|
| `AppStatePlugin` | `States` : `Boot → Loading → MainMenu → InGame(Paused?) → ...`, `SubStates` pour les overlays |
| `AssetsPlugin` | Chargement via `bevy_asset_loader` (collections typées), `GameData` en RON via `bevy_common_assets` |
| `SimPlugin` | Ressource `SimWorld`, système `FixedUpdate` qui exécute `tick()` × vitesse, file `SimCommand`, diffusion des `SimEvent` en `Message`/`Event` Bevy |
| `CameraPlugin` | Pan (clavier, glisser, bords), zoom vers le curseur, bornes, inertie (port de `camera.js`) |
| `MapRenderPlugin` | Sol et ressources par chunk, chunks visibles uniquement, vue d'ensemble sous un zoom de 0,9 |
| `BuildingRenderPlugin` | Synchronisation `SimEvent` vers les entités sprites, overlays de processeurs |
| `ItemRenderPlugin` | Rendu instancié des items sur convoyeurs et dans les éjecteurs (voir §6.3) |
| `ShapeAtlasPlugin` | Atlas dynamique des formes rastérisées, avec cache LRU |
| `PlacementPlugin` | Fantôme, rotation, variantes, glisser de convoyeurs, suppression, sélection de masse, blueprints |
| `HudPlugin` | Toolbar, objectif du hub, shop, stats, formes épinglées, notifications, paramètres |
| `WiresLayerPlugin` | Bascule de couche, rendu des wires et des pins, visualisation des valeurs |
| `AudioPlugin` | SFX et musique, volumes |
| `SavePlugin` | Autosave périodique (tâche asynchrone), liste des sauvegardes, chargement |
| `I18nPlugin` | Chargement des YAML de `translations/`, `t!("key")` |
| `DevToolsPlugin` (feature `dev`) | `bevy-inspector-egui`, overlay de performances, pas à pas des ticks, debug des `BeltPath` |

---

## 4. Workspace Cargo

```text
shapez-rs/
├── Cargo.toml                 # [workspace], resolver = "3", edition 2024, lints communs
├── LICENSE (GPL-3.0)  NOTICE  README.md
├── rust-toolchain.toml        # stable épinglée
├── .cargo/config.toml         # alias xtask, linker rapide (mold/lld), opt-level deps en dev
├── crates/
│   ├── shapez_core/           # géométrie de grille, Direction, Rotation, ShapeDef, Color, Item
│   ├── shapez_data/           # types serde des définitions (bâtiments, niveaux, upgrades) + validation
│   ├── shapez_sim/            # SimWorld, map, belts, processors, hub, wires, blueprints, commands
│   ├── shapez_save/           # format natif versionné + import des saves JS (feature "js-import")
│   ├── shapez_render_shapes/  # rastérisation ShapeDef -> RGBA (tiny-skia), indépendante de Bevy
│   └── shapez_game/           # binaire Bevy (plugins ci-dessus)
├── assets/
│   ├── data/                  # buildings.ron, levels.ron, upgrades.ron, freeplay.ron, sim.ron
│   ├── sprites/  sounds/  fonts/
│   └── i18n/                  # YAML repris de translations/
├── tools/
│   └── js-oracle/             # scripts Node qui exécutent le code JS d'origine pour générer des vecteurs de test
├── xtask/                     # import-assets, pack-atlas, gen-golden, bench-report
└── benches/                   # criterion : belts, map gen, tick complet
```

Règles :

- `shapez_core`, `shapez_data`, `shapez_sim`, `shapez_save` et `shapez_render_shapes`
  n'ont **aucune dépendance Bevy**. On peut ajouter `serde` et une feature `bevy_reflect`
  optionnelle si l'inspecteur en a besoin.
- Profil dev : `[profile.dev.package."*"] opt-level = 3` pour que Bevy reste fluide en debug.
  Activer la feature `bevy/dynamic_linking` en dev uniquement.
- Profil release : `lto = "thin"`, `codegen-units = 1` pour la distribution.

### 4.1 Choix de crates (compatibles Bevy 0.20, vérifiés le 25/11/2026)

| Besoin | Crate | Version | Pourquoi |
|---|---|---|---|
| Moteur | `bevy` | 0.20.1 | Imposé ; désactiver les features 3D inutiles (`default-features = false` + liste 2D) |
| UI MVP et outils | `bevy_egui` | 0.43 | Productivité maximale pour 48 panneaux de HUD |
| Inspecteur (dev) | `bevy-inspector-egui` | 0.38 | Debug |
| Entrées et rebinding | `leafwing-input-manager` | 0.22 | Actions, combinaisons, remapping (≈ `key_action_mapper.js`) |
| Chargement d'assets | `bevy_asset_loader` | 0.28 | États de chargement typés |
| Données RON | `bevy_common_assets` | 0.18 | `GameData` en asset, rechargement à chaud |
| Audio | audio intégré à Bevy (ou `bevy_kira_audio` 0.27 pour les fondus de musique) | — | Commencer avec l'intégré |
| Tilemap du sol | `bevy_ecs_tilemap` 0.20 **ou** maillage maison par chunk | — | Un maillage maison de 16×16 quads par chunk est simple et suffisant ; ne garder la crate que si elle simplifie |
| Rastérisation des formes | `tiny-skia` | dernière | Reproduit fidèlement les chemins du Canvas 2D (arcs, étoiles, moulins) |
| Arènes typées (simulation) | `slotmap` (avec la feature `serde`) | — | Index générationnels pour bâtiments, `BeltPath` et stores de composants, sans `Rc<RefCell>`, déterministes et sérialisables (§3.3) |
| Collections | `ahash` / `rustc-hash`, `smallvec` | — | Hash rapide **non utilisé pour l'ordre d'itération** |
| Arène temporaire (conditionnelle) | `bumpalo` | — | Allocations par tick ou par frame, **seulement si le profilage le justifie** (jalon M10, §3.3) |
| RNG | port d'**Alea** (maison, ~40 lignes) + `rand_xoshiro` | 0.8 | Alea pour la génération de carte compatible JS, Xoshiro pour le reste |
| Sérialisation | `serde`, `postcard` (1.2), `zstd`, `ron` | — | Sauvegarde binaire compacte, données lisibles |
| Import des saves JS | `lz-str`, `serde_json` | — | Décompression LZ-string et JSON |
| YAML i18n | `serde_norway` (fork maintenu de serde_yaml) | — | Lecture directe des fichiers de traduction existants |
| Union-find (wires) | maison ou `petgraph` | — | Recalcul des réseaux |
| Parallélisme (plus tard) | `rayon` | — | Mise à jour des `BeltPath` en deux phases |
| Tests | `proptest`, `insta`, `criterion` | — | Propriétés, snapshots, benchmarks |
| Erreurs et logs | `thiserror`, `tracing` (déjà dans Bevy) | — | — |

À éviter : `bevy_fluent`, qui impose Fluent (`.ftl`) alors que 34 traductions YAML existent
déjà, et `bevy_rapier` ou toute physique, inutiles ici.

---

## 5. Simulation : déterminisme et performance des convoyeurs

### 5.1 Tick fixe

- `SimConfig::TICKS_PER_SECOND = 60`. Bevy : `Time::<Fixed>::from_hz(60.0)`.
- Vitesse de jeu ×1 / ×2 / ×4 / pause : on exécute N `tick()` par pas fixe. On ne modifie
  jamais `dt`.
- Le rendu **interpole** la position des items entre deux ticks (`overstep_fraction()`)
  pour rester fluide à 144 Hz.
- Différence volontaire avec le JS : le débit sera identique quel que soit l'écran, alors
  que le tick rate dynamique de l'original le faisait légèrement varier.

### 5.2 Arithmétique entière

- Distances sur convoyeur en `u32` avec **1 tuile = 10 000 unités**. L'espacement de
  0,63 tuile vaut exactement 6 300 unités.
- Vitesse par tick d'un convoyeur = `2 items/s × multiplicateur d'upgrade × 6 300 / 60`.
  Le multiplicateur est fractionnaire : on le stocke en rationnel ou en millièmes, avec un
  **accumulateur de reste** par réseau. Aucun flottant n'intervient dans la logique.
- Les recharges de processeurs et les timers d'extracteurs sont comptés en ticks, avec
  accumulateur. Exemple : un peintre à 1/6 de la vitesse de base doit attendre
  `60 / (2 × 1/6 × mult)` ticks.
- Les flottants sont réservés au rendu (positions écran, interpolation).

### 5.3 Ordre total

- Aucune itération sur un `HashMap` dans la logique. Les itérations se font sur des `Vec`
  ou des `SlotMap` dans l'ordre d'insertion, ou sur des IDs triés.
- Ordre des systèmes identique au JS (§1.3).
- **Test de déterminisme** en CI : on construit une usine de référence, on exécute
  10 000 ticks deux fois et on compare un hash de l'état. On refait le test avec un
  aller-retour sauvegarde/chargement au tick 5 000.

### 5.4 Port de `BeltPath`

Structure de l'original (à conserver) :

- Un `BeltPath` est une suite ordonnée de tuiles de convoyeur (droites et virages)
  contiguës et de même couche.
- Items : `Vec<(distance_to_next, Item)>`, du dernier au premier. Plus
  `spacing_to_first_item`, `num_compressed_items_after_first_item` (optimisation : les
  items collés derrière un item bloqué ne sont pas recalculés) et l'accepteur final lié
  (éjecteur vers bâtiment, ou autre chemin).
- Opérations de topologie : `extend_on_end`, `extend_on_beginning`, `extend_by_path`
  (fusion), `delete_entity_on_path_split_into_two`, `delete_entity_on_start/end`. Ce sont
  les parties les plus subtiles (environ 600 lignes) : il faut les porter **avec leurs
  tests de non-régression** générés depuis le JS (§9.2).
- `update()` : distribution de la « vélocité » restante de l'avant vers l'arrière, avec
  les correctifs de perte de progression (le commentaire `#999` du JS). On les reproduit
  à l'identique.

Améliorations prévues en Rust, dans cet ordre et chacune mesurée :

1. **Layout SoA** : `distances: Vec<u32>`, `items: Vec<Item>` (items `Copy` sur 8 octets).
2. **Chemins dormants** : un chemin plein et bloqué, ou vide sans entrée, n'est plus mis
   à jour. Il est réveillé par un événement (l'accepteur se libère, un item arrive).
   C'est le gain principal sur les grosses usines, où la majorité des convoyeurs sont
   saturés.
3. **Parallélisme en deux phases** (`rayon`) : (a) chaque chemin avance ses items en
   parallèle sans toucher aux autres et produit des « offres de remise » au bout du
   chemin ; (b) une phase séquentielle, dans l'ordre des IDs, résout les remises vers les
   accepteurs. Le déterminisme est préservé. À n'activer qu'au-delà d'un seuil
   (par exemple 2 000 chemins actifs).
4. **Rendu découplé** : la simulation n'a jamais à calculer de positions monde. Le rendu
   les déduit à la demande, uniquement pour les chemins visibles.

Objectifs de performance (à valider en jalon M10) : **500 000 tuiles de convoyeur et
300 000 items à 60 ticks/s** sur une machine milieu de gamme, avec un tick sous 4 ms.

### 5.5 Autres systèmes

- **Éjecteurs et accepteurs** : cache de destination (`cachedDestSlot` ou `cachedBeltPath`
  dans le JS) recalculé uniquement pour les zones modifiées (`StaleAreaDetector`, à porter).
- **Wires** : recalcul des réseaux par parcours (union-find) seulement quand la couche
  wires est modifiée, puis évaluation de la valeur de chaque réseau à chaque tick :
  première valeur non nulle, conflit si deux valeurs diffèrent. Les portes logiques sont
  évaluées après les réseaux (ordre JS).
- **Génération de carte** : fonction pure `generate_chunk(seed, cx, cy) -> Chunk`. Avec le
  port exact d'Alea et de la chaîne de seed (`x + "|" + y + "|" + seed`, voir
  `map_chunk.js:277` ; la forme freeplay utilise `seed + "/" + level`, voir
  `hub_goals.js:403`), on obtient des cartes **identiques** aux sauvegardes JS existantes.
  Les chunks se génèrent en tâche asynchrone (`AsyncComputeTaskPool`) pour le rendu, mais
  de façon synchrone et déterministe pour la simulation.

---

## 6. Rendu

### 6.1 Couches et ordre Z

Sol (grille) → ressources → sous-couches de convoyeurs → items sur convoyeurs →
bâtiments → overlays de processeurs → couche wires (semi-transparente quand elle est
active) → fantôme de placement → UI.

### 6.2 Sprites

- Les sprites sources se trouvent dans `res_raw/sprites/**` (buildings, belt, wires,
  colors, misc, blueprints). Le JS les empaquetait avec TexturePacker en Java.
- En Rust : `xtask pack-atlas` produit un ou plusieurs atlas PNG avec leur JSON de
  rectangles (crate `texture_packer`), chargés comme `TextureAtlasLayout`. Variante MVP :
  charger les PNG individuels et utiliser le `TextureAtlasBuilder` de Bevy au chargement.
- Les sprites ont été dessinés à `assetsDpi = 6` (192 px par tuile). Prévoir des mipmaps,
  ou deux résolutions, pour le dézoom.

### 6.3 Items et formes

- Les formes ne sont **pas** des sprites : le JS les dessine en Canvas 2D (`internalGenerateShapeBuffer`).
  On porte ce dessin dans `shapez_render_shapes` avec `tiny-skia` : mêmes chemins, même
  ordre de couches, même échelle (`layerScale`). On obtient une image RGBA.
- `ShapeAtlasPlugin` : atlas dynamique (par exemple 2048² en cases de 64 px). Chaque
  `ShapeId` y est rastérisé à la première demande, avec un cache LRU. Les couleurs simples
  et les booléens sont pré-remplis.
- Rendu des items : **un seul draw instancié** par type de couche. On utilise un
  `Material2d` personnalisé avec un buffer de stockage `(position, rotation, index d'atlas)`,
  reconstruit à chaque frame uniquement à partir des chemins visibles. À défaut pour le MVP,
  un pool de `Sprite` réutilisés (pas de spawn ou despawn à chaque frame).

### 6.4 Vue d'ensemble (dézoom)

Sous un zoom de 0,9 (`mapChunkOverviewMinZoom`), le JS dessine chaque chunk comme une petite
image (couleur de la ressource ou du bâtiment par tuile). On fait de même : une texture de
16×16 pixels par chunk, mise à jour uniquement quand le chunk change, et on n'affiche plus
les items.

---

## 7. UI

- **MVP : `bevy_egui`**. Le HUD original compte 48 parties. egui permet de les produire
  vite (shop, stats, paramètres, menus, sélecteur de sauvegardes, dialogues) et de
  « skinner » les panneaux au besoin.
- **Version finale** : migrer les éléments toujours visibles et très stylés (toolbar
  du bas, objectif du hub, formes épinglées, notifications) vers `bevy_ui`, dont les
  widgets sont désormais exploitables. Les panneaux complexes peuvent rester en egui.
- Les visuels des formes dans l'UI réutilisent l'atlas dynamique (§6.3), avec
  `egui::TextureId` lié à l'image Bevy.
- Statistiques : graphiques simples dessinés avec `egui_plot` ou par un dessin maison.

---

## 8. Sauvegarde

- **Format natif** : en-tête `{magic, version_format, version_jeu, seed, temps_de_jeu, ...}`,
  puis `postcard(SaveData)` compressé en `zstd`, et un CRC32 pour détecter les corruptions.
  Pas de « sel » anti-triche, qui n'a aucun intérêt en solo.
- `SaveData` sérialise **l'état minimal** : bâtiments (code, origine, rotation, état de
  leurs composants), contenu des `BeltPath` (reconstructibles depuis les tuiles mais
  sérialisés pour conserver les items), progression, statistiques, chunks **modifiés**
  seulement (les autres sont régénérés depuis la seed).
- **Migrations** : `enum SaveVersion`, une fonction `migrate_vN_to_vN+1` par version, sur
  le modèle des schémas 1000 à 1010 du JS. Des tests de snapshot conservent une sauvegarde
  de chaque version.
- **Autosave** asynchrone : clone de `SaveData` (ou double buffer), puis sérialisation et
  écriture dans `IoTaskPool`, avec écriture atomique (fichier temporaire puis renommage).
- **Import des sauvegardes JS** (feature `js-import`, jalon M6+) : décompression
  LZ-string, vérification du CRC, `decompressObject` (dictionnaire de clés et entiers en
  base 190, voir `savegame_compressor.js`), puis lecture du schéma 1010 et conversion vers
  `SaveData`. C'est un vrai plus pour les joueurs, mais pas un prérequis.
- Emplacement : `directories::ProjectDirs`. En WASM, `localStorage` ou IndexedDB.

---

## 9. Stratégie de tests

### 9.1 Pyramide

| Niveau | Outils | Exemples |
|---|---|---|
| Unitaires (core) | `cargo test` | Parsing et formatage des codes courts, mélange de couleurs, rotation de quadrants |
| Propriétés | `proptest` | `rotate_cw⁴ = id`, `rotate_cw ∘ rotate_ccw = id`, `cut` puis recomposition, parse(format(x)) = x |
| Oracle JS (« golden ») | `tools/js-oracle` + `insta` | Mêmes entrées passées au JS et au Rust, sorties identiques (voir 9.2) |
| Scénarios de simulation | tests headless dans `shapez_sim` | « Extracteur → 10 convoyeurs → hub » : livre 0,4 item/s ±1 % ; « convoyeur saturé » : 2 items/s exactement ; fusion et scission de chemins sans perte d'item |
| Déterminisme | test CI | Hash d'état identique sur deux exécutions, et après un aller-retour de sauvegarde |
| Sauvegarde | snapshots + fuzzing (`cargo-fuzz`) | Une sauvegarde par version migrée, le chargeur ne panique jamais |
| Intégration Bevy | `App` avec `MinimalPlugins` + `SimPlugin` | Une commande de placement crée bien l'entité de rendu ; vitesse ×4 donne 4 ticks par pas |
| Performance | `criterion` + `xtask bench-report` | Tick sur des cartes de 10k, 100k et 500k tuiles, suivi des régressions en CI |

### 9.2 Oracle JS

Le code JS d'origine est la spécification. On écrit de petits scripts Node dans
`tools/js-oracle/` qui importent `shape_definition.js`, `colors.js`, `map_chunk.js`
(avec Alea) et `belt_path.js` (avec un `root` minimal simulé). Ils produisent des JSON de
vecteurs de test :

- 10 000 formes aléatoires × chaque opération : résultat attendu en code court ;
- génération des chunks (−20..20)² pour 5 seeds : ressources attendues ;
- scénarios `BeltPath` (séquences d'ajouts, suppressions et ticks) : items et espacements
  attendus. Comme le JS utilise des flottants et le Rust des entiers, on compare avec une
  tolérance d'une unité.

Ces vecteurs sont versionnés dans `crates/*/tests/golden/` : la CI n'a pas besoin de Node.

### 9.3 CI (GitHub Actions)

`fmt --check`, `clippy -D warnings`, `test --workspace`, `cargo deny` (licences
compatibles GPL et vulnérabilités), build `wasm32-unknown-unknown` (vérification
uniquement), benchmarks sur `main` uniquement.

---

## 10. Jalons

Estimations en semaines-personne pour un développeur seul à temps partiel (~10 h/semaine),
connaissant Rust et débutant sur Bevy. Ce sont des ordres de grandeur, pas des engagements.

| # | Jalon | Contenu | Critère de fin | Estimation |
|---|---|---|---|---|
| **M0** | Fondations | Workspace, licence GPLv3, CI, `xtask import-assets`, fenêtre Bevy vide, états | `cargo run` ouvre une fenêtre, CI verte | 1–2 sem |
| **M1** | Formes | `ShapeDef`, codes courts, opérations, couleurs, `ShapeRegistry`, rastérisation tiny-skia, oracle JS des formes | 100 % des vecteurs golden passent ; une galerie de formes s'affiche dans Bevy | 2–3 sem |
| **M2** | Carte et caméra | Port d'Alea, génération des chunks identique au JS, rendu du sol et des ressources par chunk, caméra pan et zoom, vue d'ensemble | Même seed que le JS, même carte (golden) ; 60 FPS en se déplaçant librement | 2–3 sem |
| **M3** | Placement | Grille d'occupation, `SimCommand::Place/Remove`, fantôme, rotation (R), variantes (T), glisser de convoyeurs avec orientation auto, suppression (X / clic droit), pipette | On peut construire des lignes de convoyeurs et des bâtiments (inactifs). Bâtiments stockés dès ce jalon dans l'arène `SlotMap<BuildingId, Building>` (§3.3) | 2–3 sem |
| **M4** | **Simulation MVP (jouable)** | Extracteurs (et chaînables), `BeltPath` complet, éjecteurs et accepteurs, tunnels T1/T2, trash, équilibreur (fusion et séparation), découpeur (et quad), rotateurs, empileur, peintres (simple, double, quad sans wires), mélangeur, hub, tick fixe et vitesses, rendu des items. `BeltPath` et stores de composants en arènes typées, références croisées en index générationnels | **On joue les niveaux 1 à 10** ; scénarios de débit et de déterminisme verts | 5–7 sem |
| **M5** | Progression et HUD | 26 niveaux et freeplay (port de `computeFreeplayShape`), shop d'upgrades et paliers infinis, notifications de déblocage, formes épinglées, statistiques, objectif du hub, tutoriel minimal | Partie complète jouable jusqu'au freeplay (sans wires) | 3–4 sem |
| **M6** | Sauvegarde | Format natif, autosave, menu de sauvegardes, migrations ; import JS en option | Sauvegarde et chargement sans perte (test de hash) ; import d'une save JS de référence (option) | 2–3 sem |
| **M7** | Confort de construction | Sélection de masse, copier-couper-coller, blueprints (coût en formes), undo/redo via les commandes, waypoints | Parité avec le JS sur ces fonctions | 2–3 sem |
| **M8** | Wires | Couche wires, wires et tunnels de wires, pins, signal constant, levier, portes ET/OU/NON/XOR, transistor, filtre, lecteur de convoyeur, afficheur, analyseur, comparateur, processeurs virtuels, peintre quad piloté | Les schémas wires classiques (filtre auto, horloges) fonctionnent | 4–5 sem |
| **M9** | Finitions | Menu principal, paramètres, rebinding, audio et musique, i18n (34 langues), thèmes clair et sombre, aide daltonien, captures d'écran | Expérience complète | 3–4 sem |
| **M10** | Performance et distribution | Chemins dormants, parallélisme, rendu instancié, benchmarks à 500k tuiles, profilage des allocations puis **introduction de `bumpalo` uniquement si le profil le justifie**, builds Windows, macOS, Linux et WASM | Objectifs du §5.4 atteints ; binaires publiés | 3–4 sem |
| Plus tard | Mode puzzle, mods | Voir §11 | — | — |

**Total jusqu'à M6 (jeu solo complet sans wires et sauvegardable) : environ 19 à 28 semaines.**

Conseils de séquencement :

- Ne pas commencer l'UI soignée avant M5. egui suffit jusque-là.
- Monter de version Bevy uniquement **entre deux jalons**, jamais en cours de jalon. Si la
  simulation est bien isolée, seule la crate `shapez_game` est concernée.
- Pour chaque bâtiment, suivre le même rythme : données RON, logique pure avec tests, puis
  rendu.

---

## 11. Hors périmètre et pistes ultérieures

- **Mode puzzle (DLC)** : il dépend d'une API en ligne propriétaire. On peut réutiliser la
  simulation (vitesses ×3, `goalAcceptor`, `constantProducer`, bâtiment `block`) avec un
  stockage local des puzzles.
- **Mods** : l'API JS n'est pas portable. Piste : des données moddables (bâtiments,
  niveaux, sous-formes) via les fichiers RON dès le départ, puis éventuellement des scripts
  WASM (`wasmtime`) pour la logique.
- **Multijoueur** : l'architecture par commandes et à tick déterministe permet un
  lockstep plus tard, sans le promettre.
- **Intégrations Steam, analytics, achievements** : facultatives (`bevy_steamworks`).

---

## 12. Risques principaux

| Risque | Impact | Mitigation |
|---|---|---|
| Comportement des convoyeurs subtilement différent (fusions, compression, correctifs `#999`) | Les usines des joueurs se comportent différemment | Oracle JS sur les scénarios `BeltPath`, port fidèle avant d'optimiser |
| Rendu des formes non identique | Aspect visuel faux | Port direct des chemins Canvas vers tiny-skia, comparaison d'images |
| Volume de l'UI (48 parties) | Retard | egui d'abord, priorisation stricte par jalon |
| Breaking changes de Bevy (~tous les 4 mois) | Temps de maintenance | Version épinglée, simulation indépendante, montées de version entre jalons |
| Licence et marque | Problème de distribution | GPLv3 dès le départ, nom distinct en cas de diffusion publique, vérification des droits sur la musique |
| Performance de rendu au dézoom | Chute des FPS | Vue d'ensemble par chunk et rendu instancié prévus dès la conception |

---

## 13. Prochaines étapes concrètes

1. Décider où vit le code Rust : nouveau dépôt (recommandé, pour un historique propre et
   éviter de mélanger avec le fork JS), ou sous-dossier `rust/` de ce fork.
2. Lancer **M0** : workspace, licence, CI, import des assets.
3. Écrire `tools/js-oracle` pour les formes, puis attaquer **M1**.
