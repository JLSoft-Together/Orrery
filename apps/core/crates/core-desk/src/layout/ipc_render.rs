use super::install;
use super::ipc::{Cached, LazySvc, Svc, Warm, WARM_PEEK};
use super::cache::render_key;
use super::xml::{clean_device, clean_diagnostics, clean_locale, clean_scale, clean_swatches, clean_text, clean_theme, clean_tree, drawable_device, drawable_kind, is_layout_target, num_or, num_or_zero, MAX_XML};
use super::{tail_chars, LayoutError, MODULE};
use crate::preview::paths::{is_absolute, is_within};
use crate::{arg, Ctx, Desk};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

const DRAWABLE_STATES: [&str; 6] = ["pressed", "focused", "selected", "checked", "activated", "disabled"];
const MAX_SLOT: i64 = 3;

fn class_missing(msg: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)class not found|NoClassDefFoundError").expect("regex")).is_match(msg)
}

pub fn uri_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn fail(e: &LayoutError) -> Value {
    let mut m = json!({"ok": false, "code": e.code});
    if let Some(d) = e.detail.as_deref().filter(|d| !d.is_empty()) {
        m["detail"] = json!(tail_chars(d, 600));
    }
    m
}

fn code(c: &str) -> Value {
    json!({"ok": false, "code": c})
}

pub(super) fn explorer(ctx: &Ctx, v: &Value) -> Option<String> {
    ctx.host("preview.explorerRoot", json!({"root": v})).ok().and_then(|x| x.as_str().map(String::from))
}

fn url_for(ctx: &Ctx, file: &str, roots: &[&str]) -> Option<String> {
    ctx.host("layout.urlFor", json!({"file": file, "roots": roots})).ok().and_then(|v| v.get("url")?.as_str().map(String::from))
}

fn real(p: &str) -> Option<String> {
    crate::preview::paths::real_or_null(p)
}

pub(super) fn head_of(file: &str) -> Option<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(file).ok()?;
    let mut buf = vec![0u8; super::xml::DRAWABLE_SNIFF_BYTES];
    let n = f.read(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf[..n]).into_owned())
}

fn load_source(file: &str, text: Option<String>) -> Result<String, Value> {
    let meta = std::fs::metadata(file).ok().filter(|m| m.is_file()).ok_or_else(|| code("missing"))?;
    if text.is_none() && meta.len() as usize > MAX_XML {
        return Err(code("too-large"));
    }
    match text {
        Some(t) => Ok(t),
        None => std::fs::read(file).map(|b| String::from_utf8_lossy(&b).into_owned()).map_err(|_| code("missing")),
    }
}

fn png_inside(svc: &Svc, res_png: &Value) -> Option<(String, String)> {
    let p = res_png.as_str().filter(|p| is_absolute(p))?;
    let render = real(&svc.render_dir)?;
    let png = real(p)?;
    (is_within(&render, &png) && Path::new(&png).extension().is_some_and(|e| e.eq_ignore_ascii_case("png"))).then_some((render, png))
}

fn snap_url(ctx: &Ctx, svc: &Svc, png: &str, tag: &str) -> Option<String> {
    let snap = real(&svc.snap_dir)?;
    url_for(ctx, png, &[&snap]).map(|u| format!("{u}?r={}", uri_component(tag)))
}

fn extra(result: &mut Map<String, Value>, prepared: &Value) {
    for k in ["frameworkOnly", "builtinLibs", "sourceOnly"] {
        result.insert(k.into(), json!(prepared[k] == json!(true)));
    }
    if prepared["prebuiltAt"].as_f64().is_some_and(|f| f != 0.0) {
        result.insert("prebuiltAt".into(), prepared["prebuiltAt"].clone());
    }
}

fn render(ctx: &Ctx, svc: &Svc, args: &[Value]) -> Value {
    let r = arg(args, 0);
    let file = explorer(ctx, &r["file"]);
    let Ok(text) = clean_text(&r["text"]) else { return code("too-large") };
    let Some(file) = file.filter(|f| is_layout_target(f, text.as_deref(), || head_of(f))) else { return code("denied") };
    let Some(device) = clean_device(&r["device"]) else { return code("bad-params") };
    if !svc.jar_exists() {
        return code("jar-missing");
    }
    if !svc.bundle.is_ready() {
        return code("bundle-missing");
    }
    let xml = match load_source(&file, text) {
        Ok(x) => x,
        Err(v) => return v,
    };
    let slot = super::xml::int_in(&r["slot"], 0, MAX_SLOT).unwrap_or(0);
    let (theme, locale, night, scale) = (clean_theme(&r["theme"]), clean_locale(&r["locale"]), r["night"] == json!(true), clean_scale(&r["scale"]));
    let prepared = svc.model.prepare(&file);
    let key = render_key(&json!([file, xml, device, theme, night, locale, scale, prepared["key"].as_str().unwrap_or("")]));
    let hit = svc.cache.lock().unwrap().get(&key);
    if let Some(hit) = hit {
        let url = hit.png.as_deref().and_then(|p| snap_url(ctx, svc, p, &key));
        if hit.png.is_none() || url.is_some() {
            let mut v = hit.result.clone();
            v["pngUrl"] = url.map(Value::from).unwrap_or(Value::Null);
            v["cached"] = json!(true);
            return v;
        }
        svc.cache.lock().unwrap().delete(&key);
    }
    let _ = std::fs::create_dir_all(&svc.render_dir);
    let params = json!({"layoutPath": file, "xmlContent": xml, "device": device, "theme": theme, "night": night, "locale": locale, "scale": scale});
    let slot_key = format!("{}:{file}:{slot}", crate::preview::gate::sender(ctx));
    let res = match svc.daemon.render(&params, &slot_key) {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    let (mut png_url, mut snap) = (None, None);
    if let Some((render_dir, png)) = png_inside(svc, &res["png"]) {
        let copy = Path::new(&svc.snap_dir).join(format!("{key}.png")).to_string_lossy().to_string();
        if std::fs::copy(&png, &copy).is_ok() {
            png_url = snap_url(ctx, svc, &copy, &key);
            snap = Some(copy);
        }
        if png_url.is_none() {
            png_url = url_for(ctx, &png, &[&render_dir]).map(|u| format!("{u}?r={}", uri_component(res["requestId"].as_str().unwrap_or(""))));
        }
    }
    let diagnostics = clean_diagnostics(&res["diagnostics"]);
    let mut result = Map::new();
    result.insert("ok".into(), json!(true));
    result.insert("width".into(), num_or(&res["width"], device["width"].as_i64().unwrap_or(0)));
    result.insert("height".into(), num_or(&res["height"], device["height"].as_i64().unwrap_or(0)));
    result.insert("viewTree".into(), clean_tree(&res["viewTree"]));
    result.insert("diagnostics".into(), diagnostics.clone());
    result.insert("renderMs".into(), num_or_zero(&res["renderMs"]));
    extra(&mut result, &prepared);
    let result = Value::Object(result);
    let needs = prepared["prebuiltAt"].as_f64().is_some_and(|f| f != 0.0) || diagnostics.as_array().is_some_and(|a| a.iter().any(|d| class_missing(d["message"].as_str().unwrap_or(""))));
    if needs {
        auto_compile(svc, &file);
    }
    if snap.is_some() || res["png"].is_null() {
        svc.cache.lock().unwrap().set(&key, Cached { result: result.clone(), png: snap });
    }
    let mut out = result;
    out["pngUrl"] = png_url.map(Value::from).unwrap_or(Value::Null);
    out["cached"] = json!(false);
    out
}

fn auto_compile(svc: &Svc, file: &str) {
    let (model, file) = (svc.model.clone(), file.to_string());
    std::thread::spawn(move || {
        model.auto_compile(&file);
    });
}

fn drawable(ctx: &Ctx, svc: &Svc, args: &[Value]) -> Value {
    let r = arg(args, 0);
    let file = explorer(ctx, &r["file"]).filter(|f| Path::new(f).extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")));
    let Some(file) = file else { return code("denied") };
    let Some(device) = drawable_device(&r["width"], &r["height"], &r["density"]) else { return code("bad-params") };
    let Ok(text) = clean_text(&r["text"]) else { return code("too-large") };
    if !svc.jar_exists() {
        return code("jar-missing");
    }
    if !svc.bundle.is_ready() {
        return code("bundle-missing");
    }
    let xml = match load_source(&file, text) {
        Ok(x) => x,
        Err(v) => return v,
    };
    let Some(kind) = drawable_kind(&file, Some(&xml)) else { return code("not-drawable") };
    let mut seen = HashSet::new();
    let states: Vec<Value> = r["states"].as_array().into_iter().flatten().filter_map(|s| s.as_str().filter(|s| DRAWABLE_STATES.contains(s) && seen.insert(s.to_string()))).map(|s| json!(s)).collect();
    let prepared = svc.model.prepare(&file);
    let _ = std::fs::create_dir_all(&svc.render_dir);
    let params = json!({"path": file, "xmlContent": xml, "device": device, "night": r["night"] == json!(true), "states": states, "colorList": kind == super::xml::DrawableKind::Color});
    let slot_key = format!("{}:{file}:drawable", crate::preview::gate::sender(ctx));
    let res = match svc.daemon.render_drawable(&params, &slot_key) {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    let png_url = png_inside(svc, &res["png"]).and_then(|(render, png)| url_for(ctx, &png, &[&render]).map(|u| format!("{u}?r={}", uri_component(res["requestId"].as_str().unwrap_or("")))));
    let mut out = Map::new();
    out.insert("ok".into(), json!(true));
    out.insert("kind".into(), json!(kind.name()));
    out.insert("pngUrl".into(), png_url.map(Value::from).unwrap_or(Value::Null));
    out.insert("width".into(), num_or(&res["width"], device["width"].as_i64().unwrap_or(0)));
    out.insert("height".into(), num_or(&res["height"], device["height"].as_i64().unwrap_or(0)));
    out.insert("swatches".into(), clean_swatches(&res["swatches"]));
    out.insert("diagnostics".into(), clean_diagnostics(&res["diagnostics"]));
    out.insert("renderMs".into(), num_or_zero(&res["renderMs"]));
    extra(&mut out, &prepared);
    Value::Object(out)
}

fn warmup(ctx: &Ctx, svc: &Svc, args: &[Value]) -> Value {
    let file = explorer(ctx, &arg(args, 0)["file"]).filter(|f| Path::new(f).extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")));
    let Some(file) = file else { return code("denied") };
    if !svc.jar_exists() {
        return code("jar-missing");
    }
    if !svc.bundle.is_ready() {
        return code("bundle-missing");
    }
    svc.model.prepare(&file);
    match svc.daemon.ensure(None) {
        Ok(()) => json!({"ok": true}),
        Err(e) => json!({"ok": false, "code": e.code}),
    }
}

fn download(ctx: &Ctx, svc: &Svc) -> Value {
    let now = svc.bundle.status();
    if matches!(now["state"].as_str(), Some("ready" | "downloading")) {
        return now;
    }
    let ok = ctx.host("layout.confirm", json!({"bytes": svc.bundle.download_bytes(), "host": svc.bundle.hosts()})).is_ok_and(|v| v == json!(true));
    if !ok {
        return json!({"state": "cancelled"});
    }
    svc.bundle.download()
}

fn uninstall(svc: &Svc) -> Value {
    let res = install::uninstall(&svc.bundle, || {
        svc.model.kill_now();
        svc.daemon.stop();
    });
    let _ = std::fs::create_dir_all(&svc.snap_dir);
    res
}

fn stage_wait(stage: Option<&str>) -> i64 {
    match stage {
        Some("jvm") => 8,
        Some("setup") => 6,
        Some("prepare") => 4,
        Some("warmup") => 2,
        _ => 10,
    }
}

fn agent_fail(e: &LayoutError) -> Value {
    let mut m = json!({"ok": false, "code": if e.code == "cancelled" { "busy" } else { e.code.as_str() }});
    if let Some(d) = e.detail.as_deref().filter(|d| !d.is_empty()) {
        m["detail"] = json!(tail_chars(d, 300));
    }
    m
}

fn agent_render(svc: &Arc<Svc>, args: &[Value]) -> Value {
    let (file, o) = (arg(args, 0).as_str().unwrap_or(""), arg(args, 1));
    if !is_layout_target(file, None, || head_of(file)) {
        return code("not-layout");
    }
    if !svc.jar_exists() {
        return code("jar-missing");
    }
    if !svc.bundle.is_ready() {
        return code("bundle-missing");
    }
    let Some(meta) = std::fs::metadata(file).ok().filter(|m| m.is_file()) else { return code("missing") };
    if meta.len() as usize > MAX_XML {
        return code("too-large");
    }
    if svc.daemon.state() != "ready" {
        let warm = {
            let mut g = svc.warm.lock().unwrap();
            match g.as_ref() {
                Some(w) => w.clone(),
                None => {
                    let w = Arc::new(Warm::default());
                    *g = Some(w.clone());
                    let (s, w2, f) = (svc.clone(), w.clone(), file.to_string());
                    std::thread::spawn(move || {
                        s.model.prepare(&f);
                        let r = s.daemon.ensure(None);
                        *w2.done.lock().unwrap() = Some(r);
                        w2.cv.notify_all();
                        *s.warm.lock().unwrap() = None;
                    });
                    w
                }
            }
        };
        let done = warm.done.lock().unwrap();
        let (done, _) = warm.cv.wait_timeout_while(done, WARM_PEEK, |d| d.is_none()).unwrap();
        match done.as_ref() {
            None => return json!({"ok": false, "code": "starting", "retryInSec": stage_wait(svc.daemon.stage().as_deref())}),
            Some(Err(e)) => return agent_fail(e),
            Some(Ok(())) => {}
        }
    }
    let Ok(xml) = std::fs::read(file).map(|b| String::from_utf8_lossy(&b).into_owned()) else { return code("missing") };
    let prepared = Some(svc.model.prepare(file));
    let _ = std::fs::create_dir_all(&svc.render_dir);
    let device = &o["device"];
    let params = json!({"layoutPath": file, "xmlContent": xml, "device": device, "theme": null, "night": o["night"], "locale": null});
    let res = match svc.daemon.render(&params, &format!("agent:{}", o["slot"].as_str().unwrap_or(""))) {
        Ok(r) => r,
        Err(e) => return agent_fail(&e),
    };
    let Some((_, png)) = png_inside(svc, &res["png"]) else { return code("render-failed") };
    json!({
        "ok": true,
        "png": png,
        "width": num_or(&res["width"], device["width"].as_i64().unwrap_or(0)),
        "height": num_or(&res["height"], device["height"].as_i64().unwrap_or(0)),
        "renderMs": num_or_zero(&res["renderMs"]),
        "frameworkOnly": prepared.map(|p| p["frameworkOnly"] == json!(true)).unwrap_or(true),
        "diagnostics": clean_diagnostics(&res["diagnostics"]),
    })
}

pub fn register(desk: &mut Desk, lazy: Arc<LazySvc>) {
    let l = lazy.clone();
    desk.on(MODULE, "ccm:layout-status", move |_, _| Ok(l.get().status()));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:layout-download", move |ctx, _| Ok(download(ctx, l.get())));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:layout-install-status", move |_, _| Ok(install::install_status(&l.get().bundle)));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:layout-uninstall", move |_, _| Ok(uninstall(l.get())));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:layout-render", move |ctx, args| Ok(render(ctx, l.get(), args)));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:drawable-render", move |ctx, args| Ok(drawable(ctx, l.get(), args)));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:layout-warmup", move |ctx, args| Ok(warmup(ctx, l.get(), args)));
    let l = lazy.clone();
    desk.on(MODULE, "desk:layout-agent-render", move |_, args| Ok(agent_render(l.get(), args)));
    let l = lazy.clone();
    desk.on(MODULE, "desk:layout-request", move |_, args| {
        let ms = arg(args, 2).as_u64().unwrap_or(120_000);
        Ok(match l.get().daemon.request(arg(args, 0).as_str().unwrap_or(""), arg(args, 1).clone(), Duration::from_millis(ms)) {
            Ok(v) => json!({"ok": true, "value": v}),
            Err(e) => json!({"ok": false, "code": e.code, "detail": e.detail}),
        })
    });
    let l = lazy.clone();
    desk.on(MODULE, "desk:layout-restart", move |_, _| {
        l.get().daemon.request_restart();
        Ok(json!(true))
    });
    let l = lazy.clone();
    desk.on(MODULE, "desk:layout-stop", move |_, _| {
        l.get().model.kill_now();
        l.get().daemon.stop();
        Ok(json!(true))
    });
    let l = lazy;
    desk.on(MODULE, "desk:layout-kill", move |_, _| {
        l.kill_now();
        Ok(json!(true))
    });
}
