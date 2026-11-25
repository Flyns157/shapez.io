// Charge des modules du jeu JS d'origine (src/js/) dans Node, sans webpack.
//
// Les `import` sont retirés et remplacés par des objets fournis par l'appelant ;
// les `export` deviennent des propriétés de l'objet renvoyé. Le code exécuté est
// donc bien celui du jeu, à l'identique, ce qui en fait l'oracle des tests Rust.

import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
// Racine `src/js` du jeu d'origine : variable SHAPEZ_JS_ROOT, sinon le dépôt parent.
export const JS_ROOT = process.env.SHAPEZ_JS_ROOT
    ? path.resolve(process.env.SHAPEZ_JS_ROOT, "src/js")
    : path.resolve(here, "../../../src/js");

/**
 * @param {string} relPath chemin relatif à src/js, par exemple "game/colors.js"
 * @param {Record<string, unknown>} scope noms disponibles pour le module (imports remplacés)
 */
export function loadModule(relPath, scope = {}) {
    let src = fs.readFileSync(path.join(JS_ROOT, relPath), "utf8");
    const exported = [];
    src = src.replace(/^import[\s\S]*?from\s+["'][^"']+["'];?\s*$/gm, "");
    src = src.replace(/^export\s+(const|let|function|class)\s+([A-Za-z0-9_$]+)/gm, (_, kind, name) => {
        exported.push(name);
        return `${kind} ${name}`;
    });
    const body = `${src}\n;({ ${exported.join(", ")} })`;
    const context = vm.createContext({
        assert: (cond, ...msg) => {
            if (!cond) throw new Error("assert: " + msg.join(" "));
        },
        assertAlways: (cond, ...msg) => {
            if (!cond) throw new Error("assert: " + msg.join(" "));
        },
        G_IS_DEV: false,
        Math,
        JSON,
        Map,
        Array,
        Object,
        Error,
        ...scope,
    });
    return vm.runInContext(body, context, { filename: relPath });
}
