//! La génération de carte doit être identique au JS (`tools/js-oracle/map.mjs`).

use std::collections::BTreeMap;

use serde::Deserialize;
use shapez_sim::map::{CHUNK_SIZE, ChunkPos, MapResource, generate_chunk};

#[derive(Deserialize)]
struct Chunk {
    seed: u32,
    cx: i32,
    cy: i32,
    items: BTreeMap<String, String>,
}

fn key(resource: MapResource) -> String {
    match resource {
        MapResource::Shape(def) => format!("shape:{def}"),
        MapResource::Color(c) => format!("color:{}", c.js_name()),
    }
}

/// Liste `x,y` des tuiles par ressource, ligne fantôme comprise, comme le JS.
fn describe(seed: u32, pos: ChunkPos) -> BTreeMap<String, String> {
    let chunk = generate_chunk(seed, pos);
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for x in 0..CHUNK_SIZE as usize {
        for y in 0..=CHUNK_SIZE as usize {
            if let Some(r) = chunk.get_with_ghost_row(x, y) {
                out.entry(key(r)).or_default().push(format!("{x},{y}"));
            }
        }
    }
    out.into_iter().map(|(k, v)| (k, v.join(" "))).collect()
}

#[test]
fn map_matches_js_oracle() {
    let data = include_str!("golden/map.json");
    let expected: Vec<Chunk> = serde_json::from_str(data).unwrap();
    let r = 15;
    let mut by_pos: BTreeMap<(u32, i32, i32), BTreeMap<String, String>> = BTreeMap::new();
    let mut seeds = Vec::new();
    for c in expected {
        if !seeds.contains(&c.seed) {
            seeds.push(c.seed);
        }
        by_pos.insert((c.seed, c.cx, c.cy), c.items);
    }
    assert_eq!(seeds.len(), 3);

    for &seed in &seeds {
        for cx in -r..=r {
            for cy in -r..=r {
                let got = describe(seed, ChunkPos::new(cx, cy));
                let want = by_pos.remove(&(seed, cx, cy)).unwrap_or_default();
                assert_eq!(got, want, "graine {seed}, chunk ({cx}, {cy})");
            }
        }
    }
    assert!(by_pos.is_empty());
}
