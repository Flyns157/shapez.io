// Vecteurs golden du générateur Alea (src/js/core/rng.js).
//
//   node rust/tools/js-oracle/rng.mjs > rust/crates/shapez_core/tests/golden/alea.json

import { loadModule } from "./load_js.mjs";

const { RandomNumberGenerator } = loadModule("core/rng.js");

const seeds = ["0", "1", "42", "0|0|0", "-3|7|12345", "99999/27", "héllo ✓", " "];
const out = seeds.map(seed => {
    const rng = new RandomNumberGenerator(seed);
    const values = [];
    for (let i = 0; i < 64; ++i) values.push(rng.next());
    const ints = [];
    for (let i = 0; i < 32; ++i) ints.push(rng.nextIntRange(-3.5, 7.5));
    return { seed, values, ints };
});
process.stdout.write(JSON.stringify(out, null, 1) + "\n");
