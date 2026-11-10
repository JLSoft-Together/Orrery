use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::process::ExitCode;

use core_procs::TcpRow;
use core_sources::mask::mask_secrets;
use core_sources::prompt_search::searcher::{PromptSearchRequest, PromptSearcher, SearcherDeps};
use core_sources::prompt_search::text::{prompt_search_terms, prompt_text_of, snippet_of};
use core_sources::resources::daemon::{daemon_of, DaemonInfo};
use core_sources::resources::ports::{ports_by_pid, ConflictCall, PortMap};
use core_sources::resources::procs::{ProcReader, ProcRow};
use core_sources::resources::sampler::{daemon_json, ResourceSampler, SampleInput};
use core_sources::resources::tree::{build_tree, TreeInput};
use core_sources::sensitive::classify::{classify_touch, ClassifyContext};
use core_sources::sensitive::settings::sanitize_sensitive;
use core_sources::sensitive::watch::{SensitiveWatch, WatchDeps};
use core_store::extra::{BackgroundJob, SensitiveSettings};
use core_store::model::{Agent, TerminalSession};
use core_transcript::rules::{command_hits, Touch};
use serde_json::{json, Value};

fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn platform(v: &Value) -> &'static str {
    match s(v) {
        "win32" => "win32",
        "darwin" => "darwin",
        _ => "linux",
    }
}

fn f64_of(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn row(v: &Value) -> ProcRow {
    let mut r = ProcRow::new(v["pid"].as_u64().unwrap_or(0) as u32, v["ppid"].as_u64().unwrap_or(0) as u32, s(&v["name"]));
    r.start = v.get("start").and_then(Value::as_str).map(str::to_owned);
    r.start_ms = v.get("startMs").and_then(f64_of);
    r.ram_bytes = v.get("ramBytes").and_then(f64_of);
    r.cpu_ms = v.get("cpuMs").and_then(f64_of);
    r.cmd = match v.get("cmd") {
        None => None,
        Some(Value::String(c)) => Some(Some(c.clone())),
        Some(_) => Some(None),
    };
    r
}

fn tcp_rows(v: &Value) -> Vec<TcpRow> {
    v.as_array()
        .map(|a| a.iter().map(|r| TcpRow { pid: r["pid"].as_u64().unwrap_or(0) as u32, state: r["state"].as_u64().unwrap_or(0) as u32, address: s(&r["address"]).to_owned(), port: r["port"].as_u64().unwrap_or(0) as u16 }).collect())
        .unwrap_or_default()
}

fn port_json(ports: &PortMap) -> Value {
    Value::Array(ports.iter().map(|(pid, list)| json!({"pid": pid, "ports": list.iter().map(core_sources::resources::sampler::port_json).collect::<Vec<_>>()})).collect())
}

fn daemon_opt(d: &Option<DaemonInfo>) -> Value {
    d.as_ref().map_or(Value::Null, daemon_json)
}

fn touch_of_item(item: &Value) -> Touch {
    match item.get("file").and_then(Value::as_str) {
        Some(f) => Touch::File(f.to_owned()),
        None => Touch::Cmds(command_hits(s(&item["cmd"]), 0)),
    }
}

fn hit_json(h: &core_sources::sensitive::classify::SensitiveHit) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("rule".into(), json!(h.rule));
    if let Some(p) = &h.path {
        m.insert("path".into(), json!(p));
    }
    if let Some(f) = &h.file {
        m.insert("file".into(), json!(f));
    }
    Value::Object(m)
}

fn settings_of(v: &Value) -> SensitiveSettings {
    sanitize_sensitive(Some(v))
}

fn classify(input: &Value) -> Value {
    let settings = settings_of(&input["settings"]);
    let safe: Vec<String> = input["safeDirs"].as_array().map(|a| a.iter().map(|d| s(d).to_owned()).collect()).unwrap_or_default();
    let ctx = ClassifyContext {
        project: input.get("project").and_then(Value::as_str),
        home: s(&input["home"]),
        win: input["win"].as_bool().unwrap_or(false),
        safe_dirs: &safe,
        settings: &settings,
        project_key: input.get("projectKey").and_then(Value::as_str),
    };
    Value::Array(input["items"].as_array().map(|a| a.iter().map(|i| Value::Array(classify_touch(&touch_of_item(i), &ctx).iter().map(hit_json).collect())).collect()).unwrap_or_default())
}

fn tree(input: &Value) -> Value {
    let rows: Vec<ProcRow> = input["rows"].as_array().map(|a| a.iter().map(row).collect()).unwrap_or_default();
    let sessions: Vec<(String, u32)> = input["sessions"].as_array().map(|a| a.iter().map(|x| (s(&x["sessionId"]).to_owned(), x["pid"].as_u64().unwrap_or(0) as u32)).collect()).unwrap_or_default();
    let lineage: HashMap<String, String> = input["lineage"].as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), s(v).to_owned())).collect()).unwrap_or_default();
    let ports = input.get("ports").filter(|p| !p.is_null()).map(|p| ports_by_pid(&tcp_rows(p)));
    let t = build_tree(&TreeInput { rows: &rows, sessions: &sessions, lineage: &lineage, ports: ports.as_ref(), keeps_parent: input["keepsParent"].as_bool().unwrap_or(false) });
    let pid = |i: usize| rows[i].pid;
    json!({
        "sessions": t.sessions.iter().map(|x| json!({"sessionId": x.session_id, "root": pid(x.root), "members": x.members.iter().map(|&m| pid(m)).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "servers": t.servers.iter().map(|e| json!({
            "root": pid(e.root), "main": pid(e.main), "members": e.members.iter().map(|&m| pid(m)).collect::<Vec<_>>(), "kind": e.kind,
            "sessionId": e.session_id, "orphan": e.orphan, "ports": e.ports.iter().map(core_sources::resources::sampler::port_json).collect::<Vec<_>>(), "daemon": daemon_opt(&e.daemon),
        })).collect::<Vec<_>>(),
        "seen": t.seen.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
    })
}

struct Passes {
    passes: Vec<Vec<ProcRow>>,
    i: usize,
}

impl ProcReader for Passes {
    fn read(&mut self) -> Option<Vec<ProcRow>> {
        let k = self.i.min(self.passes.len() - 1);
        self.i += 1;
        Some(self.passes[k].clone())
    }

    fn enrich(&mut self, _rows: &mut [ProcRow], _usage: &HashSet<u32>, _cmd: &HashSet<u32>) {}
}

fn list<T: serde::de::DeserializeOwned>(v: &Value) -> Vec<T> {
    serde_json::from_value(v.clone()).unwrap_or_default()
}

fn sample(input: &Value) -> Value {
    let passes: Vec<Vec<ProcRow>> = input["passes"].as_array().map(|a| a.iter().map(|p| p.as_array().map(|r| r.iter().map(row).collect()).unwrap_or_default()).collect()).unwrap_or_default();
    let home = s(&input["home"]).to_owned();
    let plat = platform(&input["platform"]);
    let mut sampler = ResourceSampler::new(Box::new(Passes { passes, i: 0 })).with_options(input["cores"].as_f64().unwrap_or(1.0), Box::new(move |c: &str| mask_secrets(c, &home, plat)), input["keepsParent"].as_bool().unwrap_or(false));
    let mut out = Vec::new();
    for step in input["steps"].as_array().cloned().unwrap_or_default() {
        if let Some(d) = step.get("deviceUsed").filter(|d| !d.is_null()) {
            sampler.device_used(s(&d["serial"]), d.get("avd").and_then(Value::as_str), d["at"].as_f64().unwrap_or(0.0));
        }
        let terminals: Vec<TerminalSession> = list(&step["terminals"]);
        let agents: Vec<Agent> = list(&step["agents"]);
        let jobs: Vec<BackgroundJob> = list(&step["jobs"]);
        let conflicts: Vec<ConflictCall> = step["conflicts"].as_array().map(|a| a.iter().map(|c| ConflictCall { session_id: s(&c["sessionId"]).to_owned(), port: c["port"].as_i64().unwrap_or(0), at: c["at"].as_f64().unwrap_or(0.0) }).collect()).unwrap_or_default();
        let ports = step.get("ports").filter(|p| !p.is_null()).map(|p| ports_by_pid(&tcp_rows(p)));
        let report = sampler.sample(&SampleInput {
            terminals: &terminals,
            agents: &agents,
            jobs: &jobs,
            idle_minutes: step["idleMinutes"].as_f64().unwrap_or(60.0),
            last_device_call_at: step.get("lastCall").and_then(Value::as_f64),
            conflicts: &conflicts,
            ports,
            now: step["now"].as_f64().unwrap_or(0.0),
        });
        out.push(report.unwrap_or(Value::Null));
    }
    Value::Array(out)
}

fn watch(input: &Value) -> Value {
    let now = std::sync::Arc::new(std::sync::Mutex::new(0.0f64));
    let clock = now.clone();
    let deps = WatchDeps {
        claude_root: s(&input["claudeRoot"]).into(),
        home: s(&input["home"]).to_owned(),
        tmp: s(&input["tmp"]).into(),
        win: input["win"].as_bool().unwrap_or(false),
        now: Some(Box::new(move || *clock.lock().unwrap())),
        root_of: None,
    };
    let file = input.get("file").and_then(Value::as_str).map(std::path::PathBuf::from);
    let mut w = SensitiveWatch::new(deps, file.clone());
    w.load();
    let mut out = Vec::new();
    for op in input["ops"].as_array().cloned().unwrap_or_default() {
        match s(&op["op"]) {
            "check" => {
                *now.lock().unwrap() = op["now"].as_f64().unwrap_or(0.0);
                let hits = w.check(s(&op["terminalId"]), &touch_of_item(&op["touch"]), op.get("cwd").and_then(Value::as_str), op.get("at").and_then(Value::as_f64));
                out.push(Value::Array(
                    hits.iter()
                        .map(|h| {
                            let mut v = hit_json(&h.hit);
                            if let Some(p) = &h.project {
                                v["project"] = json!(p);
                            }
                            v
                        })
                        .collect(),
                ));
            }
            "ignore" => out.push(json!(w.ignore(s(&op["project"]), s(&op["rule"]), op["on"].as_bool().unwrap_or(false)))),
            "update" => {
                w.update(op.get("groups").filter(|v| !v.is_null()), op.get("globs").filter(|v| !v.is_null()), op.get("ignore").filter(|v| !v.is_null()));
                out.push(Value::Null);
            }
            _ => out.push(Value::Null),
        }
    }
    w.flush();
    json!({"results": out, "settings": serde_json::to_value(w.settings()).unwrap_or(Value::Null)})
}

fn prompts(input: &Value) -> Value {
    let queries: Vec<String> = input["queries"].as_array().map(|a| a.iter().map(|q| s(q).to_owned()).collect()).unwrap_or_default();
    let texts: Vec<String> = input["texts"].as_array().map(|a| a.iter().map(|q| s(q).to_owned()).collect()).unwrap_or_default();
    let records: Vec<Value> = input["records"].as_array().cloned().unwrap_or_default();
    json!({
        "terms": queries.iter().map(|q| prompt_search_terms(q)).collect::<Vec<_>>(),
        "snippets": texts.iter().map(|t| queries.iter().map(|q| snippet_of(t, &prompt_search_terms(q), 200)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "records": records.iter().map(|r| prompt_text_of(r).map_or(Value::Null, |t| json!(t))).collect::<Vec<_>>(),
        "mask": texts.iter().map(|t| mask_secrets(t, s(&input["home"]), platform(&input["platform"]))).collect::<Vec<_>>(),
    })
}

fn search(input: &Value) -> Value {
    let now = input["now"].as_f64().unwrap_or(0.0);
    let deps = SearcherDeps {
        now: Some(Box::new(move || now)),
        home: s(&input["home"]).to_owned(),
        platform: platform(&input["platform"]),
        open_folder: None,
        progress_ms: 0.0,
        yield_fn: Some(Box::new(|| {})),
    };
    let searcher = PromptSearcher::new(s(&input["projectsDir"]), deps);
    let mut events = Vec::new();
    let req = PromptSearchRequest {
        search_id: "x".into(),
        query: s(&input["query"]).to_owned(),
        days: input["days"].as_u64().unwrap_or(0) as u32,
        project: input.get("project").and_then(Value::as_str).map(str::to_owned),
    };
    searcher.start(1, &req, &mut |e| events.push(e));
    Value::Array(events)
}

fn main() -> ExitCode {
    let Some(cmd) = std::env::args().nth(1) else { return ExitCode::from(2) };
    let mut text = String::new();
    if std::io::stdin().read_to_string(&mut text).is_err() {
        return ExitCode::from(2);
    }
    let Ok(input) = serde_json::from_str::<Value>(&text) else { return ExitCode::from(2) };
    let out = match cmd.as_str() {
        "daemon" => Value::Array(input.as_array().map(|a| a.iter().map(|p| daemon_opt(&daemon_of(s(&p[0]), p[1].as_str()))).collect()).unwrap_or_default()),
        "classify" => classify(&input),
        "tree" => tree(&input),
        "sample" => sample(&input),
        "watch" => watch(&input),
        "prompts" => prompts(&input),
        "search" => search(&input),
        "ports" => port_json(&ports_by_pid(&tcp_rows(&input))),
        _ => return ExitCode::from(2),
    };
    println!("{out}");
    ExitCode::SUCCESS
}
