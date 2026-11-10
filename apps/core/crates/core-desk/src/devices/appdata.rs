use super::manager::DeviceManager;
use super::parse::{err, err_detail, is_ok};
use super::prefs::{apply_pref_edit, parse_prefs, pref_shown, PrefEdit};
use super::sqlguard::{check_read_only_sql, quote_ident};
use super::sqlwal::{apply_wal, is_sqlite};
use super::toggles::sh_quote;
use crate::apk::analyze::locale_cmp;
use crate::js;
use crate::run::android::jsre::{g, Js};
use crate::util::now_ms;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, MAIN_DB};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const APPDATA_ORIGINALS_DIR: &str = "appdata-originals";
pub const DB_MAX: usize = 128 * 1024 * 1024;
pub const WAL_MAX: usize = 64 * 1024 * 1024;
pub const PREFS_MAX: usize = 2 * 1024 * 1024;
pub const ROWS_PAGE: usize = 200;
pub const QUERY_ROWS_MAX: usize = 1000;
const CELL_MAX: usize = 4000;
const OPEN_MAX: usize = 3;
const TABLES_MAX: usize = 300;
const PULL_MS: u64 = 60_000;
const CMD_MS: u64 = 15_000;
const LIST_SCRIPT: &str = "for f in databases/* shared_prefs/*; do [ -f \"$f\" ] && echo \"$(wc -c < \"$f\") $f\"; done; true";

pub fn is_data_file_name(v: &str) -> bool {
    static NAME: Js = Js::new(r"^(?![.-])[A-Za-z0-9._+-]{1,128}$", "");
    NAME.test(v)
}

pub fn parse_data_list(out: &str) -> (Vec<Value>, Vec<Value>, u64) {
    static LINE: Js = Js::new(r"^\s*(\d+)\s+(databases|shared_prefs)\/(.+?)\s*$", "");
    static SIDE: Js = Js::new(r"-(wal|shm|journal)$", "");
    let mut db_names: Vec<(String, u64)> = Vec::new();
    let mut prefs: Vec<(String, u64)> = Vec::new();
    let mut skipped = 0;
    for line in out.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let Some(m) = LINE.exec(line) else { continue };
        let (size, dir, name) = (g(&m, 1).unwrap_or("0").parse::<u64>().unwrap_or(0), g(&m, 2).unwrap_or(""), g(&m, 3).unwrap_or("").to_string());
        if !is_data_file_name(&name) {
            skipped += 1;
            continue;
        }
        if dir == "databases" {
            match db_names.iter_mut().find(|(n, _)| *n == name) {
                Some(e) => e.1 = size,
                None => db_names.push((name, size)),
            }
        } else if name.ends_with(".xml") {
            prefs.push((name, size));
        }
    }
    let mut databases: Vec<(String, u64, bool)> = db_names.iter().filter(|(n, _)| !SIDE.test(n)).map(|(n, s)| (n.clone(), *s, db_names.iter().any(|(o, _)| *o == format!("{n}-wal")))).collect();
    databases.sort_by(|a, b| locale_cmp(&a.0, &b.0));
    prefs.sort_by(|a, b| locale_cmp(&a.0, &b.0));
    let dbs = databases.into_iter().map(|(n, s, w)| if w { json!({"name": n, "size": s, "wal": true}) } else { json!({"name": n, "size": s}) }).collect();
    (dbs, prefs.into_iter().map(|(n, s)| json!({"name": n, "size": s})).collect(), skipped)
}

pub fn run_as_failure(out: &str) -> Option<Value> {
    static NOT_DEBUG: Js = Js::new(r"not debuggable", "i");
    static UNKNOWN: Js = Js::new(r"unknown package|is unknown|not an application|package .* not found", "i");
    static RUNAS: Js = Js::new(r"run-as:", "i");
    static WS: Js = Js::new(r"\s+", "g");
    if NOT_DEBUG.test(out) {
        return Some(err("not-debuggable"));
    }
    if UNKNOWN.test(out) {
        return Some(err("unknown-package"));
    }
    if RUNAS.test(out) {
        return Some(err_detail("denied", &js::slice(WS.replace_all(out, " ").trim(), 200)));
    }
    None
}

fn fail(code: &str, detail: &str) -> Value {
    err_detail(code, detail)
}

fn sqlite_message(e: &rusqlite::Error) -> String {
    match e {
        rusqlite::Error::SqlInputError { msg, .. } => msg.clone(),
        rusqlite::Error::SqliteFailure(_, Some(m)) => m.clone(),
        other => other.to_string(),
    }
}

fn cell(v: ValueRef) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => super::history::num(f),
        ValueRef::Text(t) => {
            let s = String::from_utf8_lossy(t).to_string();
            if js::len(&s) > CELL_MAX {
                json!(format!("{}…", js::slice(&s, CELL_MAX)))
            } else {
                json!(s)
            }
        }
        ValueRef::Blob(b) => json!({"blob": b.len()}),
    }
}

type Tables = (Vec<Value>, HashMap<String, Option<i64>>);

struct OpenDb {
    conn: Connection,
    counts: HashMap<String, Option<i64>>,
    used: f64,
}

pub struct AppData {
    m: Arc<DeviceManager>,
    data_dir: PathBuf,
    open: Mutex<HashMap<String, OpenDb>>,
    writing: AtomicBool,
}

fn rand_hex(n: usize) -> String {
    let mut raw = vec![0u8; n];
    let _ = getrandom::fill(&mut raw);
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

impl AppData {
    pub fn new(m: Arc<DeviceManager>, data_dir: &std::path::Path) -> Self {
        AppData { m, data_dir: data_dir.to_path_buf(), open: Mutex::new(HashMap::new()), writing: AtomicBool::new(false) }
    }

    fn original_of(&self, serial: &str, pkg: &str, name: &str) -> PathBuf {
        let hex: String = Sha1::digest(serial.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        self.data_dir.join(APPDATA_ORIGINALS_DIR).join(hex).join(pkg).join(name)
    }

    fn with_device(&self, id: &str, f: impl FnOnce() -> Value) -> Value {
        let ready = self.m.ready_check(id);
        if is_ok(&ready) {
            f()
        } else {
            ready
        }
    }

    fn run_as(&self, id: &str, pkg: &str) -> Option<Value> {
        let Some(r) = self.m.adb.shell(id, &["run-as".into(), sh_quote(pkg), "true".into()], Some(CMD_MS)) else { return Some(err("adb-missing")) };
        if r.timed_out {
            return Some(err("timeout"));
        }
        run_as_failure(&r.out).or_else(|| {
            (r.code != Some(0)).then(|| {
                let d = js::slice(r.out.trim(), 200);
                err_detail("denied", &d)
            })
        })
    }

    fn pull(&self, id: &str, pkg: &str, rel: &str, max: usize) -> Result<Vec<u8>, Value> {
        static GONE: Js = Js::new(r"No such file", "i");
        let args = ["run-as".to_string(), sh_quote(pkg), "cat".to_string(), sh_quote(rel)];
        let Some(r) = self.m.adb.exec_out(id, &args, PULL_MS, max) else { return Err(err("adb-missing")) };
        if r.timed_out {
            return Err(err("timeout"));
        }
        if r.overflow {
            return Err(err("too-large"));
        }
        let head = String::from_utf8_lossy(&r.out[..r.out.len().min(300)]).to_string();
        let text = format!("{head}{}", r.err);
        if r.out.len() < 300 && GONE.test(&text) {
            return Err(err("gone"));
        }
        if r.out.len() < 300 {
            if let Some(refused) = run_as_failure(&text) {
                return Err(refused);
            }
        }
        if r.code.is_none() {
            return Err(err("pull-failed"));
        }
        Ok(r.out)
    }

    pub fn list(&self, id: &str, pkg: &str) -> Value {
        self.with_device(id, || {
            if let Some(refused) = self.run_as(id, pkg) {
                return refused;
            }
            let args = ["run-as".to_string(), sh_quote(pkg), "sh".to_string(), "-c".to_string(), sh_quote(LIST_SCRIPT)];
            let Some(r) = self.m.adb.shell(id, &args, Some(CMD_MS)) else { return err("adb-missing") };
            if r.timed_out {
                return err("timeout");
            }
            let (databases, prefs, skipped) = parse_data_list(&r.out);
            json!({"ok": true, "pkg": pkg, "databases": databases, "prefs": prefs, "skipped": skipped})
        })
    }

    fn close_oldest(map: &mut HashMap<String, OpenDb>) {
        while map.len() >= OPEN_MAX {
            let Some(h) = map.iter().min_by(|a, b| a.1.used.partial_cmp(&b.1.used).unwrap_or(std::cmp::Ordering::Equal)).map(|(h, _)| h.clone()) else { return };
            map.remove(&h);
        }
    }

    fn load_tables(conn: &Connection) -> Result<Tables, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT name, type, sql FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY type, name")?;
        let rows: Vec<(String, String, Option<String>)> = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)))?.collect::<Result<_, _>>()?;
        let (mut tables, mut counts) = (Vec::new(), HashMap::new());
        for (name, ty, sql) in rows.into_iter().take(TABLES_MAX) {
            let count = if ty == "table" { conn.query_row(&format!("SELECT count(*) FROM {}", quote_ident(&name)), [], |r| r.get::<_, i64>(0)).ok() } else { None };
            counts.insert(name.clone(), count);
            tables.push(json!({"name": name, "type": if ty == "view" { "view" } else { "table" }, "sql": sql.unwrap_or_default(), "rows": count}));
        }
        Ok((tables, counts))
    }

    pub fn open_db(&self, id: &str, pkg: &str, name: &str) -> Value {
        self.with_device(id, || {
            if let Some(refused) = self.run_as(id, pkg) {
                return refused;
            }
            let main = match self.pull(id, pkg, &format!("databases/{name}"), DB_MAX) {
                Ok(b) => b,
                Err(e) => return e,
            };
            let wal_res = self.pull(id, pkg, &format!("databases/{name}-wal"), WAL_MAX);
            let wal = wal_res.as_ref().ok().filter(|w| !w.is_empty());
            let wal_missing = wal_res.as_ref().err().is_some_and(|e| e["error"] != json!("gone"));
            let applied = apply_wal(&main, wal.map(Vec::as_slice));
            if !is_sqlite(&applied.db) {
                return err("not-sqlite");
            }
            let mut conn = match Connection::open_in_memory() {
                Ok(c) => c,
                Err(e) => return fail("not-sqlite", &js::slice(&sqlite_message(&e), 200)),
            };
            if let Err(e) = conn.deserialize_read_exact(MAIN_DB, std::io::Cursor::new(&applied.db), applied.db.len(), true) {
                return fail("not-sqlite", &js::slice(&sqlite_message(&e), 200));
            }
            let loaded = conn.execute_batch("PRAGMA query_only = ON").and_then(|_| Self::load_tables(&conn));
            let (tables, counts) = match loaded {
                Ok(t) => t,
                Err(e) => return fail("not-sqlite", &js::slice(&sqlite_message(&e), 200)),
            };
            let handle = rand_hex(8);
            {
                let mut open = self.open.lock().unwrap();
                Self::close_oldest(&mut open);
                open.insert(handle.clone(), OpenDb { conn, counts, used: now_ms() });
            }
            let state = if applied.partial || wal_missing { "partial" } else if applied.frames > 0 { "applied" } else { "none" };
            json!({"ok": true, "handle": handle, "name": name, "size": main.len(), "tables": tables, "wal": state, "walFrames": applied.frames})
        })
    }

    fn run(o: &mut OpenDb, sql: &str, max: usize) -> Value {
        o.used = now_ms();
        let result = (|| {
            let mut stmt = o.conn.prepare(sql)?;
            let columns: Vec<String> = stmt.column_names().map(|c| c.to_string()).collect();
            let n = columns.len();
            let mut rows_out: Vec<Value> = Vec::new();
            let mut truncated = false;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                if rows_out.len() >= max {
                    truncated = true;
                    break;
                }
                rows_out.push(Value::Array((0..n).map(|i| row.get_ref(i).map(cell).unwrap_or(Value::Null)).collect()));
            }
            Ok::<Value, rusqlite::Error>(json!({"ok": true, "columns": columns, "rows": rows_out, "total": null, "truncated": truncated}))
        })();
        result.unwrap_or_else(|e| fail("sql-error", &js::slice(&sqlite_message(&e), 300)))
    }

    pub fn rows(&self, handle: &str, table: &str, page: usize) -> Value {
        let mut open = self.open.lock().unwrap();
        let Some(o) = open.get_mut(handle) else { return err("gone") };
        let Some(total) = o.counts.get(table).copied() else { return err("invalid") };
        let mut r = Self::run(o, &format!("SELECT * FROM {} LIMIT {ROWS_PAGE} OFFSET {}", quote_ident(table), page * ROWS_PAGE), ROWS_PAGE);
        if is_ok(&r) {
            r["total"] = total.map(|t| json!(t)).unwrap_or(Value::Null);
            r["truncated"] = json!(false);
        }
        r
    }

    pub fn query(&self, handle: &str, sql: &Value) -> Value {
        let mut open = self.open.lock().unwrap();
        let Some(o) = open.get_mut(handle) else { return err("gone") };
        match check_read_only_sql(sql) {
            Ok(s) => Self::run(o, &s, QUERY_ROWS_MAX),
            Err(reason) => fail("sql-rejected", reason),
        }
    }

    pub fn close(&self, handle: &str) {
        self.open.lock().unwrap().remove(handle);
    }

    pub fn close_all(&self) {
        self.open.lock().unwrap().clear();
    }

    fn prefs_of(&self, id: &str, pkg: &str, name: &str) -> Result<(Vec<u8>, String), Value> {
        let bytes = self.pull(id, pkg, &format!("shared_prefs/{name}"), PREFS_MAX)?;
        let text = String::from_utf8_lossy(&bytes).to_string();
        Ok((bytes, text))
    }

    fn has_original(&self, id: &str, pkg: &str, name: &str) -> bool {
        self.original_of(id, pkg, name).is_file()
    }

    fn prefs_result(&self, id: &str, pkg: &str, name: &str) -> Value {
        let (_, text) = match self.prefs_of(id, pkg, name) {
            Ok(r) => r,
            Err(e) => return e,
        };
        match parse_prefs(&text) {
            Some(entries) => json!({"ok": true, "name": name, "entries": entries, "hasOriginal": self.has_original(id, pkg, name)}),
            None => err("pull-failed"),
        }
    }

    pub fn read_prefs(&self, id: &str, pkg: &str, name: &str) -> Value {
        self.with_device(id, || self.run_as(id, pkg).unwrap_or_else(|| self.prefs_result(id, pkg, name)))
    }

    fn push(&self, id: &str, pkg: &str, name: &str, bytes: &[u8]) -> Option<Value> {
        if self.m.adb.shell(id, &["am".into(), "force-stop".into(), sh_quote(pkg)], Some(CMD_MS)).is_none() {
            return Some(err("adb-missing"));
        }
        let tmp = format!("shared_prefs/{name}.orrery-tmp");
        let script = sh_quote(&format!("cat > {tmp} && mv {tmp} shared_prefs/{name}"));
        let Some(w) = self.m.adb.shell_input(id, &["run-as".into(), sh_quote(pkg), "sh".into(), "-c".into(), script], bytes, CMD_MS) else { return Some(err("adb-missing")) };
        if w.timed_out {
            return Some(err("timeout"));
        }
        match self.pull(id, pkg, &format!("shared_prefs/{name}"), PREFS_MAX) {
            Ok(back) if back == bytes => None,
            _ => Some(err_detail("write-failed", &js::slice(format!("{} {}", String::from_utf8_lossy(&w.out), w.err).trim(), 200))),
        }
    }

    fn exclusive(&self, f: impl FnOnce() -> Value) -> Value {
        if self.writing.swap(true, Ordering::SeqCst) {
            return fail("cancelled", "busy");
        }
        let r = f();
        self.writing.store(false, Ordering::SeqCst);
        r
    }

    pub fn write_pref(&self, id: &str, pkg: &str, name: &str, edit: &PrefEdit) -> Value {
        self.exclusive(|| {
            self.with_device(id, || {
                if let Some(refused) = self.run_as(id, pkg) {
                    return refused;
                }
                let (cur_bytes, cur_text) = match self.prefs_of(id, pkg, name) {
                    Ok(c) => c,
                    Err(e) => return e,
                };
                let next = match apply_pref_edit(&cur_text, edit) {
                    Ok(x) => x,
                    Err(e) => return err(e),
                };
                let ask = json!({"kind": "pref-write", "pkg": pkg, "file": name, "key": edit.key, "before": pref_shown(&edit.before), "after": pref_shown(&edit.after)});
                if !matches!(self.m.adb.host.call("devices.confirm", ask), Ok(Value::Bool(true))) {
                    return err("cancelled");
                }
                let original = self.original_of(id, pkg, name);
                if !original.is_file() {
                    use std::io::Write;
                    let saved = original.parent().map(std::fs::create_dir_all).transpose().and_then(|_| std::fs::OpenOptions::new().write(true).create_new(true).open(&original)).and_then(|mut f| f.write_all(&cur_bytes));
                    if saved.is_err() {
                        return fail("write-failed", "original");
                    }
                }
                self.push(id, pkg, name, next.as_bytes()).unwrap_or_else(|| self.prefs_result(id, pkg, name))
            })
        })
    }

    pub fn restore_prefs(&self, id: &str, pkg: &str, name: &str) -> Value {
        self.exclusive(|| {
            self.with_device(id, || {
                let original = self.original_of(id, pkg, name);
                let Ok(bytes) = std::fs::read(&original) else { return err("no-original") };
                if let Some(refused) = self.run_as(id, pkg) {
                    return refused;
                }
                if !matches!(self.m.adb.host.call("devices.confirm", json!({"kind": "pref-restore", "pkg": pkg, "file": name})), Ok(Value::Bool(true))) {
                    return err("cancelled");
                }
                if let Some(e) = self.push(id, pkg, name, &bytes) {
                    return e;
                }
                let _ = std::fs::remove_file(&original);
                self.prefs_result(id, pkg, name)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_databases_with_wal_and_prefs() {
        let out = "100 databases/a.db\n20 databases/a.db-wal\n5 databases/a.db-shm\n9 shared_prefs/p.xml\n3 shared_prefs/bad name.xml\n7 databases/.hidden\n";
        let (dbs, prefs, skipped) = parse_data_list(out);
        assert_eq!(dbs, vec![json!({"name": "a.db", "size": 100, "wal": true})]);
        assert_eq!(prefs, vec![json!({"name": "p.xml", "size": 9})]);
        assert_eq!(skipped, 2);
        assert_eq!(run_as_failure("run-as: package not debuggable: x").unwrap()["error"], json!("not-debuggable"));
    }
}
