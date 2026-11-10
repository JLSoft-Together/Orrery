use super::gate::{self, denied, Latest};
use super::history::{LocalHistory, NotesStore};
use super::{changes, explorer, files, search};
use crate::{arg, Ctx, Desk, DeskEnv};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

pub const MODULE: &str = "previewFiles";

fn field<'a>(req: &'a Value, key: &str) -> &'a Value {
    req.get(key).unwrap_or(&Value::Null)
}

fn strings(v: Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn roots(ctx: &Ctx) -> Vec<String> {
    strings(ctx.host("preview.roots", json!({})).unwrap_or(Value::Null))
}

fn write_roots(ctx: &Ctx) -> Vec<String> {
    strings(ctx.host("preview.writeRoots", json!({})).unwrap_or(Value::Null))
}

fn editable(ctx: &Ctx, file: &Value) -> Option<String> {
    ctx.host("preview.editableFile", json!({"file": file})).ok()?.get("real")?.as_str().map(String::from)
}

fn explorer_root(ctx: &Ctx, root: &Value) -> Option<String> {
    ctx.host("preview.explorerRoot", json!({"root": root})).ok()?.as_str().map(String::from)
}

fn is_dir(p: &str) -> bool {
    std::fs::metadata(p).map(|m| m.is_dir()).unwrap_or(false)
}

fn history_on(ctx: &Ctx) -> bool {
    gate::has_cap(&gate::caps(ctx), "history")
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    desk.on(MODULE, "ccm:read-file", |ctx, args| {
        let t = arg(args, 0);
        if !t.is_string() {
            return Ok(Value::Null);
        }
        Ok(files::read_file_for_view(t, &roots(ctx)))
    });
    desk.on(MODULE, "ccm:log-read", |ctx, args| {
        let req = arg(args, 0);
        if !req.is_object() && !req.is_array() {
            return Ok(denied());
        }
        Ok(files::read_log_chunk(req, &roots(ctx)))
    });
    let history = Arc::new(LocalHistory::new(env.data_dir.clone()));
    let h = history.clone();
    desk.on(MODULE, "ccm:file-write", move |ctx, args| {
        let on = history_on(ctx);
        Ok(files::write_text_file(arg(args, 0), &write_roots(ctx), &|real, prev| {
            if on {
                h.record(real, prev);
            }
        }))
    });
    let h = history.clone();
    desk.on(MODULE, "ccm:lh-list", move |ctx, args| {
        if !history_on(ctx) {
            return Ok(denied());
        }
        Ok(editable(ctx, arg(args, 0)).map(|real| h.list(&real)).unwrap_or_else(denied))
    });
    let h = history.clone();
    desk.on(MODULE, "ccm:lh-read", move |ctx, args| {
        let req = arg(args, 0);
        if !history_on(ctx) {
            return Ok(denied());
        }
        Ok(editable(ctx, field(req, "path")).map(|real| h.read(&real, field(req, "id"))).unwrap_or_else(denied))
    });
    desk.on(MODULE, "ccm:list-dir", |ctx, args| {
        let req = arg(args, 0);
        Ok(explorer_root(ctx, field(req, "root")).map(|r| explorer::list_dir(&r, field(req, "dir"))).unwrap_or(Value::Null))
    });
    desk.on(MODULE, "ccm:create-entry", |ctx, args| {
        let req = arg(args, 0);
        let Some(real) = explorer_root(ctx, field(req, "root")) else { return Ok(denied()) };
        let res = explorer::create_entry(&real, field(req, "parent"), field(req, "name"), field(req, "kind"));
        if res["ok"] == json!(true) {
            gate::invalidate(ctx, &real);
        }
        Ok(res)
    });
    desk.on(MODULE, "ccm:rename-entry", |ctx, args| {
        let req = arg(args, 0);
        let Some(real) = explorer_root(ctx, field(req, "root")) else { return Ok(denied()) };
        let res = explorer::rename_entry(&real, field(req, "path"), field(req, "name"));
        if res["ok"] == json!(true) {
            gate::invalidate(ctx, &real);
        }
        Ok(res)
    });
    desk.on(MODULE, "ccm:trash-entry", |ctx, args| {
        let req = arg(args, 0);
        let Some(real) = explorer_root(ctx, field(req, "root")) else { return Ok(denied()) };
        if !gate::has_cap(&gate::caps(ctx), "trash") {
            return Ok(json!({"ok": false, "error": "failed"}));
        }
        let res = explorer::trash_entry(&real, field(req, "path"), &|p| gate::trash(ctx, p));
        if res["ok"] == json!(true) {
            gate::invalidate(ctx, &real);
        }
        Ok(res)
    });
    let searches = Arc::new(Latest::default());
    desk.on(MODULE, "ccm:search", move |ctx, args| {
        let req = arg(args, 0);
        let Some(q) = search::clean_query(field(req, "query")) else { return Ok(json!({"ok": false, "error": "bad-query"})) };
        let Some(ok) = gate::allowed(ctx, field(req, "dir"), false).filter(|a| is_dir(&a.real)) else { return Ok(denied()) };
        let key = gate::sender(ctx).to_string();
        let flag = searches.start(&key);
        let res = search::search(&ok.real, &q, flag.clone());
        searches.finish(&key, &flag);
        Ok(res)
    });
    desk.on(MODULE, "ccm:commands-list", |ctx, args| {
        let Some(ok) = gate::allowed(ctx, arg(args, 0), false) else { return Ok(denied()) };
        let mut list = vec![ok.real.clone()];
        if let Ok(top) = changes::toplevel(&ok.real) {
            if top != ok.real {
                list.push(top);
            }
        }
        let mut dirs: Vec<(PathBuf, &str)> = list.iter().map(|r| (PathBuf::from(r).join(".claude").join("commands"), "project")).collect();
        dirs.push((search::user_commands_dir(), "user"));
        Ok(json!({"ok": true, "commands": search::list_commands(&dirs)}))
    });
    let notes = Arc::new(NotesStore::new(env.data_dir.clone()));
    let n = notes.clone();
    desk.on(MODULE, "ccm:notes-read", move |ctx, args| {
        if !history_on(ctx) {
            return Ok(denied());
        }
        Ok(gate::check_dir(ctx, arg(args, 0)).map(|r| n.read(&r)).unwrap_or_else(denied))
    });
    desk.on(MODULE, "ccm:notes-write", move |ctx, args| {
        let req = arg(args, 0);
        if !history_on(ctx) {
            return Ok(denied());
        }
        Ok(gate::check_dir(ctx, field(req, "root")).map(|r| notes.write(&r, field(req, "text"))).unwrap_or_else(denied))
    });
}
