use super::http::{Fetch, Fetched};
use super::osv::{bare_vuln, osv_batch_url, osv_ecosystem, osv_vuln_url, parse_osv_batch, parse_osv_vuln};
use super::registry::{lookup_url, parse_maven_metadata, parse_npm_versions, parse_pub_versions, registries_for};
use super::versions::{eco_of, is_lib_name, is_lib_version, pick_latest, LIBS_MAX};
use crate::{js, util};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub const LIBS_CACHE_FILE: &str = "lib-versions-cache.json";
pub const LIBS_TTL_MS: f64 = 60.0 * 60.0 * 1000.0;
pub const LIBS_CONCURRENCY: usize = 6;
const CACHE_MAX: usize = 3000;
const BODY_MAX: usize = 32 * 1024 * 1024;
pub const OSV_DETAILS_MAX: usize = 300;

#[derive(Default)]
struct Cache {
    loaded: bool,
    entries: Map<String, Value>,
    vulns: Map<String, Value>,
}

pub struct LibChecker {
    fetch: Box<dyn Fetch>,
    data_dir: PathBuf,
    over: Option<String>,
    osv_over: Option<String>,
    cache: Mutex<Cache>,
    save_lock: Mutex<()>,
}

#[derive(Clone)]
struct Query {
    eco: &'static str,
    name: String,
    version: Option<String>,
}

impl Query {
    fn key(&self) -> String {
        match &self.version {
            Some(v) => format!("{}:{}@{}", self.eco, self.name, v),
            None => format!("{}:{}", self.eco, self.name),
        }
    }

    fn head(&self) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("eco".into(), json!(self.eco));
        m.insert("name".into(), json!(self.name));
        if let Some(v) = &self.version {
            m.insert("version".into(), json!(v));
        }
        m
    }
}

fn with(mut m: Map<String, Value>, extra: Value) -> Value {
    if let Value::Object(e) = extra {
        for (k, v) in e {
            m.insert(k, v);
        }
    }
    Value::Object(m)
}

fn at_of(v: &Value) -> Option<f64> {
    v.as_object().and_then(|o| o.get("at")).and_then(Value::as_f64)
}

fn accept_for(eco: &str) -> &'static str {
    match eco {
        "npm" => "application/vnd.npm.install-v1+json",
        "pub" => "application/vnd.pub.v2+json",
        "osv" => "application/json",
        _ => "application/xml",
    }
}

fn parse_queries(input: &Value, versioned: bool) -> Result<Vec<Query>, Value> {
    let Some(list) = input.as_array() else { return Err(json!({"ok": false, "error": "invalid"})) };
    if list.len() > LIBS_MAX {
        return Err(json!({"ok": false, "error": "too-many"}));
    }
    let mut out: Vec<Query> = Vec::new();
    let mut seen = HashSet::new();
    for raw in list {
        let field = |k: &str| raw.as_object().and_then(|o| o.get(k)).cloned().unwrap_or(Value::Null);
        let (eco, name, version) = (field("eco"), field("name"), field("version"));
        if !is_lib_name(&eco, &name) || (versioned && !is_lib_version(&version)) {
            return Err(json!({"ok": false, "error": "invalid"}));
        }
        let q = Query {
            eco: eco_of(&eco).unwrap_or("npm"),
            name: name.as_str().unwrap_or_default().to_string(),
            version: if versioned { version.as_str().map(str::to_string) } else { None },
        };
        if seen.insert(q.key()) {
            out.push(q);
        }
    }
    Ok(out)
}

fn workers<T: Sync>(items: &[T], f: impl Fn(&T) + Sync) {
    let next = AtomicUsize::new(0);
    let n = LIBS_CONCURRENCY.min(items.len());
    std::thread::scope(|s| {
        for _ in 0..n {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(item) = items.get(i) else { break };
                f(item);
            });
        }
    });
}

impl LibChecker {
    pub fn new(fetch: Box<dyn Fetch>, data_dir: PathBuf, over: Option<String>, osv_over: Option<String>) -> Self {
        LibChecker { fetch, data_dir, over, osv_over, cache: Mutex::new(Cache::default()), save_lock: Mutex::new(()) }
    }

    fn file(&self) -> PathBuf {
        self.data_dir.join(LIBS_CACHE_FILE)
    }

    fn load_cache(&self) {
        let mut c = self.cache.lock().unwrap();
        if c.loaded {
            return;
        }
        c.loaded = true;
        let Ok(text) = std::fs::read(self.file()) else { return };
        let Ok(Value::Object(doc)) = serde_json::from_slice::<Value>(&text) else { return };
        if let Some(Value::Object(e)) = doc.get("entries") {
            for (k, v) in js::entries(e) {
                if at_of(v).is_some() {
                    c.entries.insert(k.clone(), v.clone());
                }
            }
        }
        if let Some(Value::Object(e)) = doc.get("vulns") {
            for (k, v) in js::entries(e) {
                if at_of(v).is_some() && v.get("vulns").is_some_and(Value::is_array) {
                    c.vulns.insert(k.clone(), v.clone());
                }
            }
        }
    }

    fn save(&self) {
        let body = {
            let mut guard = self.cache.lock().unwrap();
            let c = &mut *guard;
            let cutoff = util::now_ms() - LIBS_TTL_MS;
            for m in [&mut c.entries, &mut c.vulns] {
                m.retain(|_, v| at_of(v).is_none_or(|at| at >= cutoff));
                while m.len() > CACHE_MAX {
                    let Some(first) = m.keys().next().cloned() else { break };
                    m.shift_remove(&first);
                }
            }
            js::stringify(&json!({"entries": c.entries, "vulns": c.vulns}))
        };
        let _guard = self.save_lock.lock().unwrap();
        let _ = std::fs::create_dir_all(&self.data_dir);
        let tmp = self.data_dir.join(format!("{LIBS_CACHE_FILE}.tmp"));
        if std::fs::write(&tmp, body).is_ok() {
            let _ = std::fs::rename(&tmp, self.file());
        }
    }

    fn fetch_text(&self, url: &str, eco: &str, body: Option<&str>) -> Fetched {
        match self.fetch.fetch(url, accept_for(eco), body) {
            Fetched::Text(t) if js::len(&t) > BODY_MAX => Fetched::Status(413),
            other => other,
        }
    }

    fn lookup(&self, q: &Query) -> Value {
        let mut last = "not-found";
        for registry in registries_for(q.eco, &q.name) {
            let Some(url) = lookup_url(registry, q.eco, &q.name, self.over.as_deref()) else {
                return with(q.head(), json!({"ok": false, "error": "invalid"}));
            };
            let text = match self.fetch_text(&url, q.eco, None) {
                Fetched::Error(e) => return with(q.head(), json!({"ok": false, "error": e})),
                Fetched::Status(s) => {
                    last = if s == 404 || s == 403 { "not-found" } else { "bad-response" };
                    continue;
                }
                Fetched::Text(t) => t,
            };
            let versions = match q.eco {
                "maven" => parse_maven_metadata(&text),
                "pub" => parse_pub_versions(&text),
                _ => parse_npm_versions(&text),
            };
            let Some(versions) = versions else {
                return with(q.head(), json!({"ok": false, "error": "bad-response"}));
            };
            let latest = pick_latest(&versions);
            return with(q.head(), json!({"ok": true, "registry": registry, "stable": latest["stable"], "pre": latest["pre"], "checkedAt": util::now_ms()}));
        }
        with(q.head(), json!({"ok": false, "error": last}))
    }

    pub fn check(&self, input: &Value, fresh: bool) -> Value {
        let queries = match parse_queries(input, false) {
            Ok(q) => q,
            Err(e) => return e,
        };
        self.load_cache();
        let mut out: HashMap<String, Value> = HashMap::new();
        let mut todo: Vec<Query> = Vec::new();
        {
            let c = self.cache.lock().unwrap();
            let now = util::now_ms();
            for q in &queries {
                match c.entries.get(&q.key()).and_then(|hit| at_of(hit).map(|at| (hit, at))) {
                    Some((hit, at)) if !fresh && now - at < LIBS_TTL_MS => {
                        out.insert(q.key(), with(q.head(), json!({"ok": true, "registry": hit["registry"], "stable": hit["stable"], "pre": hit["pre"], "checkedAt": hit["at"]})));
                    }
                    _ => todo.push(q.clone()),
                }
            }
        }
        let found = Mutex::new(out);
        workers(&todo, |q| {
            let r = self.lookup(q);
            if r["ok"] == Value::Bool(true) {
                let entry = json!({"at": r["checkedAt"], "registry": r["registry"], "stable": r["stable"], "pre": r["pre"]});
                self.cache.lock().unwrap().entries.insert(q.key(), entry);
            }
            found.lock().unwrap().insert(q.key(), r);
        });
        if !todo.is_empty() {
            self.save();
        }
        let out = found.into_inner().unwrap();
        json!({"ok": true, "results": queries.iter().map(|q| out.get(&q.key()).cloned().unwrap_or(Value::Null)).collect::<Vec<_>>()})
    }

    pub fn vulns(&self, input: &Value, fresh: bool) -> Value {
        let queries = match parse_queries(input, true) {
            Ok(q) => q,
            Err(e) => return e,
        };
        self.load_cache();
        let mut out: HashMap<String, Value> = HashMap::new();
        let mut todo: Vec<Query> = Vec::new();
        {
            let c = self.cache.lock().unwrap();
            let now = util::now_ms();
            for q in &queries {
                match c.vulns.get(&q.key()).and_then(|hit| at_of(hit).map(|at| (hit, at))) {
                    Some((hit, at)) if !fresh && now - at < LIBS_TTL_MS => {
                        out.insert(q.key(), with(q.head(), json!({"ok": true, "vulns": hit["vulns"], "checkedAt": hit["at"]})));
                    }
                    _ => todo.push(q.clone()),
                }
            }
        }
        if !todo.is_empty() {
            match self.fetch_details(&todo) {
                Ok(found) => out.extend(found),
                Err(e) => return json!({"ok": false, "error": e}),
            }
            self.save();
        }
        json!({"ok": true, "results": queries.iter().map(|q| out.get(&q.key()).cloned().unwrap_or(Value::Null)).collect::<Vec<_>>()})
    }

    fn fetch_details(&self, todo: &[Query]) -> Result<Vec<(String, Value)>, &'static str> {
        let base = self.osv_over.as_deref();
        let queries: Vec<Value> = todo
            .iter()
            .map(|q| json!({"package": {"ecosystem": osv_ecosystem(q.eco), "name": q.name}, "version": q.version}))
            .collect();
        let body = js::stringify(&json!({"queries": queries}));
        let ids = match self.fetch_text(&osv_batch_url(base), "osv", Some(&body)) {
            Fetched::Error(e) => return Err(e),
            Fetched::Text(t) => parse_osv_batch(&t, todo.len()),
            Fetched::Status(_) => None,
        };
        let ids = ids.ok_or("bad-response")?;
        let mut owners: Vec<(String, Vec<usize>)> = Vec::new();
        for (i, list) in ids.iter().enumerate() {
            for id in list {
                match owners.iter_mut().find(|(x, _)| x == id) {
                    Some((_, qs)) => qs.push(i),
                    None => owners.push((id.clone(), vec![i])),
                }
            }
        }
        owners.truncate(OSV_DETAILS_MAX);
        let details: Mutex<HashMap<String, HashMap<String, Value>>> = Mutex::new(HashMap::new());
        let failed: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        workers(&owners, |(id, qs)| {
            let r = self.fetch_text(&osv_vuln_url(id, base), "osv", None);
            let mut per = HashMap::new();
            for &i in qs {
                let q = &todo[i];
                let v = match &r {
                    Fetched::Text(t) => parse_osv_vuln(t, q.eco, &q.name, q.version.as_deref().unwrap_or_default()),
                    _ => None,
                };
                match v {
                    Some(v) => {
                        per.insert(q.key(), v);
                    }
                    None if !matches!(r, Fetched::Text(_)) => {
                        failed.lock().unwrap().insert(q.key());
                    }
                    None => {}
                }
            }
            details.lock().unwrap().insert(id.clone(), per);
        });
        let details = details.into_inner().unwrap();
        let failed = failed.into_inner().unwrap();
        let at = util::now_ms();
        let mut out = Vec::new();
        let mut cache = self.cache.lock().unwrap();
        for (i, q) in todo.iter().enumerate() {
            let k = q.key();
            let vulns: Vec<Value> = ids[i]
                .iter()
                .flat_map(|id| match details.get(id) {
                    None => vec![bare_vuln(id)],
                    Some(d) => match d.get(&k) {
                        Some(v) => vec![v.clone()],
                        None if failed.contains(&k) => vec![bare_vuln(id)],
                        None => vec![],
                    },
                })
                .collect();
            if !failed.contains(&k) && ids[i].iter().all(|id| details.contains_key(id)) {
                cache.vulns.insert(k.clone(), json!({"at": at, "vulns": vulns}));
            }
            out.push((k, with(q.head(), json!({"ok": true, "vulns": vulns, "checkedAt": at}))));
        }
        Ok(out)
    }
}
