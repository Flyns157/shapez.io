// Vecteurs golden de la génération de carte (src/js/game/map_chunk.js).
//
//   node rust/tools/js-oracle/map.mjs > rust/crates/shapez_sim/tests/golden/map.json
//
// Pour chaque graine et chaque chunk de (-R..R)², on enregistre les ressources de la
// couche basse : un objet par item, avec un masque des tuiles qu'il occupe. Le masque
// couvre 16 × 17 tuiles car le JS accepte y == 16 (bug d'origine, reproduit en Rust).

import { loadModule } from "./load_js.mjs";

const globalConfig = { mapChunkSize: 16, tileSize: 32, mapChunkWorldSize: 512 };
const utils = {
    clamp: (v, minimum = 0, maximum = 1) => Math.max(minimum, Math.min(maximum, v)),
    make2DUndefinedArray: (w, h) => {
        const result = new Array(w);
        for (let x = 0; x < w; ++x) result[x] = new Array(h);
        return result;
    },
    safeModulo: (n, m) => ((n % m) + m) % m,
    fastArrayDeleteValueIfContained: () => {},
};
const { Vector } = loadModule("core/vector.js", { globalConfig, ...utils });
const { RandomNumberGenerator } = loadModule("core/rng.js");
const colors = loadModule("game/colors.js");
const { ShapeDefinition, enumSubShape } = loadModule("game/shape_definition.js", {
    ...colors,
    BasicSerializableObject: class {},
    Vector,
    globalConfig,
    THEME: {},
    types: {},
    makeOffscreenBuffer: null,
    smoothenDpi: x => x,
    DrawParameters: class {},
});

const COLOR_ITEM_SINGLETONS = {};
for (const c of Object.values(colors.enumColors)) COLOR_ITEM_SINGLETONS[c] = { key: "color:" + c };

const shapeDefinitionMgr = {
    getDefinitionFromSimpleShapes: (subShapes, color = colors.enumColors.uncolored) =>
        new ShapeDefinition({ layers: [subShapes.map(subShape => ({ subShape, color }))] }),
    getShapeItemFromDefinition: def => ({ key: "shape:" + def.getHash() }),
    getShapeItemFromShortKey: key => ({ key: "shape:" + key }),
};

const { MapChunk } = loadModule("game/map_chunk.js", {
    globalConfig,
    ...utils,
    Vector,
    RandomNumberGenerator,
    enumColors: colors.enumColors,
    COLOR_ITEM_SINGLETONS,
    enumSubShape,
    createLogger: () => ({ error: console.error, warn: () => {}, log: () => {} }),
    Rectangle: class {},
    BaseItem: class {},
    Entity: class {},
    GameRoot: class {},
});

const R = Number(process.argv[2] || 15);
const seeds = [0, 1234, 99999];
const out = [];
for (const seed of seeds) {
    const root = { map: { seed }, shapeDefinitionMgr };
    for (let cx = -R; cx <= R; ++cx) {
        for (let cy = -R; cy <= R; ++cy) {
            const chunk = new MapChunk(root, cx, cy);
            const masks = new Map();
            for (let x = 0; x < 16; ++x) {
                for (let y = 0; y <= 16; ++y) {
                    const item = chunk.lowerLayer[x][y];
                    if (!item) continue;
                    if (!masks.has(item.key)) masks.set(item.key, []);
                    masks.get(item.key).push(x + "," + y);
                }
            }
            if (masks.size === 0) continue;
            out.push({ seed, cx, cy, items: Object.fromEntries([...masks].map(([k, v]) => [k, v.join(" ")])) });
        }
    }
}
process.stdout.write("[\n" + out.map(c => JSON.stringify(c)).join(",\n") + "\n]\n");
