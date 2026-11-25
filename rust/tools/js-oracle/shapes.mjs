// Génère les vecteurs golden des formes à partir du code JS d'origine.
//
//   node rust/tools/js-oracle/shapes.mjs [N=2000] > rust/crates/shapez_core/tests/golden/shapes.json
//
// Chaque cas contient une forme aléatoire (graine fixe) et le résultat, en code court,
// de chaque opération de ShapeDefinition (découpe, rotations, empilement, peinture).

import { loadModule } from "./load_js.mjs";

const colors = loadModule("game/colors.js");

class BasicSerializableObject {}
class Vector {
    constructor(x = 0, y = 0) {
        this.x = x;
        this.y = y;
    }
}
const { ShapeDefinition, enumSubShape } = loadModule("game/shape_definition.js", {
    ...colors,
    BasicSerializableObject,
    Vector,
    globalConfig: {},
    THEME: {},
    types: {},
    makeOffscreenBuffer: () => {
        throw new Error("non utilisé");
    },
    smoothenDpi: x => x,
    DrawParameters: class {},
});

// mulberry32 : générateur seedé, pour que le fichier soit reproductible.
function mulberry32(seed) {
    return () => {
        seed |= 0;
        seed = (seed + 0x6d2b79f5) | 0;
        let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}
const rand = mulberry32(0x5ead);
const pick = arr => arr[Math.floor(rand() * arr.length)];

const subShapes = Object.values(enumSubShape);
const allColors = Object.values(colors.enumColors);

function randomLayer(fill) {
    const quads = [null, null, null, null];
    while (quads.every(q => q === null)) {
        for (let q = 0; q < 4; ++q) {
            quads[q] = rand() < fill ? { subShape: pick(subShapes), color: pick(allColors) } : null;
        }
    }
    return quads;
}

function randomShape() {
    const layerCount = 1 + Math.floor(rand() * 4);
    const fill = pick([0.25, 0.5, 0.75, 1]);
    const layers = [];
    for (let i = 0; i < layerCount; ++i) {
        layers.push(randomLayer(fill));
    }
    return new ShapeDefinition({ layers });
}

const N = Number(process.argv[2] || 2000);
const cases = [];
for (let i = 0; i < N; ++i) {
    const shape = randomShape();
    const other = randomShape();
    const paint = pick(allColors);
    const paint4 = [0, 1, 2, 3].map(() => (rand() < 0.3 ? null : pick(allColors)));
    const mix = [pick(allColors), pick(allColors)];
    cases.push({
        shape: shape.getHash(),
        other: other.getHash(),
        cut: [shape.cloneFilteredByQuadrants([2, 3]), shape.cloneFilteredByQuadrants([0, 1])].map(s =>
            s.getHash()
        ),
        cutQuad: [0, 1, 2, 3].map(q => shape.cloneFilteredByQuadrants([q]).getHash()),
        cw: shape.cloneRotateCW().getHash(),
        ccw: shape.cloneRotateCCW().getHash(),
        r180: shape.cloneRotate180().getHash(),
        stack: shape.cloneAndStackWith(other).getHash(),
        paint,
        painted: shape.cloneAndPaintWith(paint).getHash(),
        paint4,
        painted4: shape.cloneAndPaintWith4Colors(paint4).getHash(),
        valid: ShapeDefinition.isValidShortKey(shape.getHash()),
        mix,
        mixed: colors.enumColorMixingResults[mix[0]][mix[1]],
    });
}

// Une ligne par cas : le fichier reste lisible et les diffs restent courts.
process.stdout.write("[\n" + cases.map(c => JSON.stringify(c)).join(",\n") + "\n]\n");
