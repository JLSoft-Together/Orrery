use crate::collate::locale_cmp;
use crate::index::{Scan, ScanState, UsageIndex};
use crate::omap::Num;
use crate::record::Totals;
use crate::time::last_days;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};

pub fn scan_json(s: &Scan) -> Value {
    let state = match s.state {
        ScanState::Idle => "idle",
        ScanState::Scanning => "scanning",
        ScanState::Ready => "ready",
    };
    json!({ "state": state, "filesDone": s.files_done, "filesTotal": s.files_total })
}

fn n(v: f64) -> Value {
    serde_json::to_value(Num(v)).unwrap_or(Value::Null)
}

pub fn bucket_json(key: &str, t: &Totals) -> Value {
    let parts: Vec<&str> = key.split('|').collect();
    let day = parts.first().copied().unwrap_or("");
    let model = parts.get(1).copied().unwrap_or("");
    let mut rest: Vec<&str> = parts.iter().skip(2).copied().collect();
    let speed = rest.pop();
    let mut m = Map::new();
    m.insert("key".into(), key.into());
    m.insert("day".into(), day.into());
    m.insert("model".into(), model.into());
    m.insert("project".into(), rest.join("|").into());
    if speed == Some("f") {
        m.insert("fast".into(), true.into());
    }
    m.insert("messages".into(), n(t.messages));
    m.insert("input".into(), n(t.input));
    m.insert("output".into(), n(t.output));
    m.insert("cacheWrite5m".into(), n(t.cache_write_5m));
    m.insert("cacheWrite1h".into(), n(t.cache_write_1h));
    m.insert("cacheRead".into(), n(t.cache_read));
    Value::Object(m)
}

#[derive(Default, Clone)]
struct Tally {
    list: Vec<(String, f64)>,
    at: HashMap<String, usize>,
}

impl Tally {
    fn add(&mut self, key: &str, n: f64) {
        match self.at.get(key) {
            Some(&i) => self.list[i].1 += n,
            None => {
                self.at.insert(key.to_owned(), self.list.len());
                self.list.push((key.to_owned(), n));
            }
        }
    }

    fn get(&self, key: &str) -> f64 {
        self.at.get(key).map_or(0.0, |&i| self.list[i].1)
    }
}

fn by_count(a: &(String, f64), b: &(String, f64)) -> std::cmp::Ordering {
    b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
}

impl UsageIndex {
    pub fn snapshot_json(&self) -> Value {
        let buckets: Vec<Value> = self.totals.iter().map(|(k, t)| bucket_json(k, t)).collect();
        json!({ "buckets": buckets, "scan": scan_json(&self.scan) })
    }

    pub fn take_dirty(&mut self) -> Vec<Value> {
        let out = self.dirty.iter().filter_map(|(k, _)| self.totals.get(k).map(|t| bucket_json(k, t))).collect();
        self.dirty.clear();
        out
    }

    pub fn heatmap(&self, days: u32, today: &str) -> Value {
        let rows: Vec<Value> = last_days(days, today)
            .into_iter()
            .map(|day| {
                let hours: Vec<Value> = (0..24).map(|h| n(self.hour_totals.get(&format!("{day}|{h}")).copied().unwrap_or(0.0))).collect();
                json!({ "day": day, "hours": hours })
            })
            .collect();
        json!({ "days": rows })
    }

    pub fn tool_stats(&self, days: u32, project: Option<&str>, today: &str) -> Value {
        let month: HashSet<String> = last_days(30, today).into_iter().collect();
        let window: HashSet<String> = last_days(days, today).into_iter().collect();
        let mut per_project = Tally::default();
        let mut sums: HashMap<char, Tally> = HashMap::new();
        let mut used_s: HashSet<String> = HashSet::new();
        let mut used_m: HashSet<String> = HashSet::new();
        for (key, &count) in self.tool_totals.iter() {
            let parts: Vec<&str> = key.split('|').collect();
            let day = parts.first().copied().unwrap_or("");
            let kind = parts.get(1).copied().unwrap_or("");
            let name = parts.get(2).copied().unwrap_or("");
            let Some(kc) = (match kind {
                "t" | "m" | "s" | "a" | "e" | "f" => kind.chars().next(),
                _ => None,
            }) else {
                continue;
            };
            if !month.contains(day) {
                continue;
            }
            let proj = parts.get(3..).map(|r| r.join("|")).unwrap_or_default();
            if kc == 't' || kc == 'm' {
                per_project.add(&proj, count);
            }
            if project.is_some_and(|p| p != proj) {
                continue;
            }
            if kc == 's' {
                used_s.insert(name.to_owned());
            }
            if kc == 'm' {
                used_m.insert(name.to_owned());
            }
            if !window.contains(day) {
                continue;
            }
            sums.entry(kc).or_default().add(name, count);
        }
        let lookup = |k: char, name: &str| sums.get(&k).map_or(0.0, |t| t.get(name));
        let list = |k: char, err: Option<char>| -> Value {
            let mut items: Vec<(String, f64)> = sums.get(&k).map(|t| t.list.clone()).unwrap_or_default();
            items.sort_by(|a, b| by_count(a, b).then_with(|| locale_cmp(&a.0, &b.0)));
            items.truncate(50);
            Value::Array(
                items
                    .into_iter()
                    .map(|(name, c)| match err {
                        Some(e) => json!({ "name": name, "n": n(c), "errors": n(lookup(e, &name)) }),
                        None => json!({ "name": name, "n": n(c) }),
                    })
                    .collect(),
            )
        };
        let mut per_project = per_project.list;
        per_project.sort_by(by_count);
        let projects: Vec<String> = per_project.into_iter().take(100).map(|(p, _)| p).collect();
        let sorted = |s: HashSet<String>| {
            let mut v: Vec<String> = s.into_iter().collect();
            v.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            v
        };
        json!({
            "days": days,
            "project": project,
            "projects": projects,
            "tools": list('t', Some('e')),
            "skills": list('s', None),
            "agents": list('a', None),
            "mcp": list('m', Some('f')),
            "used30": { "skills": sorted(used_s), "mcp": sorted(used_m) },
        })
    }
}
