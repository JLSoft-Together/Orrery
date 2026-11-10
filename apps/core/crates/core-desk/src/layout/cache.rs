use serde_json::Value;
use sha1::{Digest, Sha1};
use std::collections::VecDeque;

pub struct RenderCache<T> {
    order: VecDeque<(String, T)>,
    max: usize,
    on_evict: Box<dyn Fn(&T) + Send>,
}

impl<T: Clone> RenderCache<T> {
    pub fn new(max: usize, on_evict: Box<dyn Fn(&T) + Send>) -> Self {
        RenderCache { order: VecDeque::new(), max, on_evict }
    }

    pub fn get(&mut self, key: &str) -> Option<T> {
        let at = self.order.iter().position(|(k, _)| k == key)?;
        let entry = self.order.remove(at)?;
        let v = entry.1.clone();
        self.order.push_back(entry);
        Some(v)
    }

    pub fn set(&mut self, key: &str, value: T) {
        if let Some(at) = self.order.iter().position(|(k, _)| k == key) {
            if let Some((_, old)) = self.order.remove(at) {
                (self.on_evict)(&old);
            }
        }
        self.order.push_back((key.to_string(), value));
        while self.order.len() > self.max {
            if let Some((_, v)) = self.order.pop_front() {
                (self.on_evict)(&v);
            }
        }
    }

    pub fn delete(&mut self, key: &str) {
        if let Some(at) = self.order.iter().position(|(k, _)| k == key) {
            if let Some((_, v)) = self.order.remove(at) {
                (self.on_evict)(&v);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

pub fn render_key(parts: &Value) -> String {
    let digest = Sha1::digest(crate::js::stringify(parts).as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[test]
    fn lru_evicts_oldest_and_refreshes_on_get() {
        let gone = Arc::new(Mutex::new(Vec::new()));
        let g = gone.clone();
        let mut c = RenderCache::new(2, Box::new(move |v: &i32| g.lock().unwrap().push(*v)));
        c.set("a", 1);
        c.set("b", 2);
        assert_eq!(c.get("a"), Some(1));
        c.set("c", 3);
        assert_eq!(*gone.lock().unwrap(), vec![2]);
        c.set("a", 9);
        assert_eq!(*gone.lock().unwrap(), vec![2, 1]);
        c.delete("c");
        assert_eq!((c.len(), c.get("c")), (1, None));
    }

    #[test]
    fn key_changes_with_inputs() {
        let d = json!({"width": 1, "height": 2, "density": 3});
        let k = |night: bool, m: &str| render_key(&json!(["/a", "<x/>", d, null, night, null, 1.0, m]));
        let a = k(false, "m");
        assert_eq!(a, k(false, "m"));
        assert_ne!(a, k(true, "m"));
        assert_ne!(a, k(false, "m2"));
    }
}
