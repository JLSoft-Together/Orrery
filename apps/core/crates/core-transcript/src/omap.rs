use serde::ser::{Serialize, SerializeMap, Serializer};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OMap<V> {
    slots: Vec<Option<(String, V)>>,
    index: HashMap<String, usize>,
}

impl<V> Default for OMap<V> {
    fn default() -> Self {
        OMap { slots: Vec::new(), index: HashMap::new() }
    }
}

impl<V> OMap<V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn slot_of(&self, key: &str) -> Option<usize> {
        self.index.get(key).copied()
    }

    pub fn get(&self, key: &str) -> Option<&V> {
        self.slot_of(key).and_then(|i| self.at(i))
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let i = self.slot_of(key)?;
        self.at_mut(i)
    }

    pub fn at(&self, slot: usize) -> Option<&V> {
        self.slots.get(slot).and_then(|s| s.as_ref()).map(|(_, v)| v)
    }

    pub fn at_mut(&mut self, slot: usize) -> Option<&mut V> {
        self.slots.get_mut(slot).and_then(|s| s.as_mut()).map(|(_, v)| v)
    }

    pub fn key_at(&self, slot: usize) -> Option<&str> {
        self.slots.get(slot).and_then(|s| s.as_ref()).map(|(k, _)| k.as_str())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    pub fn insert(&mut self, key: &str, value: V) -> usize {
        if let Some(i) = self.slot_of(key) {
            if let Some((_, v)) = self.slots[i].as_mut() {
                *v = value;
            }
            return i;
        }
        let i = self.slots.len();
        self.slots.push(Some((key.to_owned(), value)));
        self.index.insert(key.to_owned(), i);
        i
    }

    pub fn entry_or(&mut self, key: &str, make: impl FnOnce() -> V) -> &mut V {
        let i = match self.slot_of(key) {
            Some(i) => i,
            None => self.insert(key, make()),
        };
        self.at_mut(i).expect("live slot")
    }

    pub fn remove(&mut self, key: &str) -> Option<V> {
        let i = self.index.remove(key)?;
        let out = self.slots[i].take().map(|(_, v)| v);
        if self.slots.len() > 64 && self.slots.len() > 2 * self.index.len() {
            self.compact();
        }
        out
    }

    fn compact(&mut self) {
        self.slots.retain(Option::is_some);
        self.index.clear();
        for (i, s) in self.slots.iter().enumerate() {
            if let Some((k, _)) = s {
                self.index.insert(k.clone(), i);
            }
        }
    }

    pub fn first_key(&self) -> Option<&str> {
        self.slots.iter().flatten().next().map(|(k, _)| k.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.slots.iter().flatten().map(|(k, v)| (k.as_str(), v))
    }

    pub fn iter_slots(&self) -> impl Iterator<Item = (usize, &str, &V)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| s.as_ref().map(|(k, v)| (i, k.as_str(), v)))
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.index.clear();
    }
}

impl<V: Serialize> Serialize for OMap<V> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.len()))?;
        for (k, v) in self.iter() {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Num(pub f64);

impl Serialize for Num {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let v = self.0;
        if v.fract() == 0.0 && v.abs() < 9_007_199_254_740_992.0 {
            s.serialize_i64(v as i64)
        } else if v.is_finite() {
            s.serialize_f64(v)
        } else {
            s.serialize_unit()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_insertion_order_and_moves_reinserted_keys_last() {
        let mut m = OMap::new();
        m.insert("a", 1);
        m.insert("b", 2);
        m.insert("a", 3);
        assert_eq!(m.iter().collect::<Vec<_>>(), vec![("a", &3), ("b", &2)]);
        m.remove("a");
        m.insert("a", 4);
        assert_eq!(m.iter().collect::<Vec<_>>(), vec![("b", &2), ("a", &4)]);
        assert_eq!(m.first_key(), Some("b"));
        for i in 0..200 {
            m.insert(&format!("k{i}"), i);
        }
        for i in 0..190 {
            m.remove(&format!("k{i}"));
        }
        assert_eq!(m.len(), 12);
        assert_eq!(m.get("k195"), Some(&195));
        assert_eq!(m.first_key(), Some("b"));
    }

    #[test]
    fn numbers_print_like_json_stringify() {
        assert_eq!(serde_json::to_string(&Num(5.0)).unwrap(), "5");
        assert_eq!(serde_json::to_string(&Num(1730000000123.5)).unwrap(), "1730000000123.5");
        assert_eq!(serde_json::to_string(&Num(-0.0)).unwrap(), "0");
    }
}
