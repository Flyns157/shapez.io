//! Interning des formes : chaque `ShapeDef` distincte reçoit un `ShapeId` stable.

use std::collections::HashMap;

use crate::ShapeDef;

/// Identifiant d'une forme internée, valable pour le [`ShapeRegistry`] qui l'a créé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShapeId(u32);

impl ShapeId {
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Table `ShapeDef <-> ShapeId`. Les identifiants sont attribués dans l'ordre
/// d'insertion, ce qui garde l'ordre d'itération déterministe.
#[derive(Debug, Default, Clone)]
pub struct ShapeRegistry {
    defs: Vec<ShapeDef>,
    ids: HashMap<ShapeDef, ShapeId>,
}

impl ShapeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renvoie l'identifiant de la forme, en l'enregistrant si besoin.
    pub fn intern(&mut self, def: ShapeDef) -> ShapeId {
        if let Some(&id) = self.ids.get(&def) {
            return id;
        }
        let id = ShapeId(u32::try_from(self.defs.len()).expect("plus de 2^32 formes"));
        self.defs.push(def);
        self.ids.insert(def, id);
        id
    }

    pub fn get(&self, id: ShapeId) -> &ShapeDef {
        &self.defs[id.index()]
    }

    pub fn find(&self, def: &ShapeDef) -> Option<ShapeId> {
        self.ids.get(def).copied()
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// Formes dans l'ordre d'enregistrement.
    pub fn iter(&self) -> impl Iterator<Item = (ShapeId, &ShapeDef)> {
        self.defs
            .iter()
            .enumerate()
            .map(|(i, d)| (ShapeId(i as u32), d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_is_stable() {
        let mut reg = ShapeRegistry::new();
        let a: ShapeDef = "CuCuCuCu".parse().unwrap();
        let b = a.rotate_cw();
        let c = a.paint(crate::Color::Red);
        let ia = reg.intern(a);
        assert_eq!(
            reg.intern(b),
            ia,
            "une rotation d'une forme symétrique est la même forme"
        );
        let ic = reg.intern(c);
        assert_ne!(ia, ic);
        assert_eq!(reg.get(ic), &c);
        assert_eq!(reg.len(), 2);
        assert_eq!(
            reg.iter().map(|(id, _)| id).collect::<Vec<_>>(),
            vec![ia, ic]
        );
    }
}
