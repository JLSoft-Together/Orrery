use super::host::Args;
use super::registry::{resolve_dir, Registry, Ticket, MAX_LIVE, OPEN_RATE_MAX, OPEN_RATE_MS, OPEN_REPLY_MS, SPAWN_WAIT_MS, TICKET_TTL_MS};
use super::t_open::to_open_result;
use super::text::{clean_pane_prompt, is_agent_session_id, is_preset};
use crate::util::uuid;
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::OnceLock;

pub fn is_pane_name(v: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    v.trim() == v && R.get_or_init(|| Regex::new(r"^[\p{L}\p{N}][\p{L}\p{N} ._-]{0,39}$").unwrap()).is_match(v)
}

fn err(code: &str) -> Value {
    json!({ "ok": false, "error": code })
}

impl Registry {
    pub fn open_from(&self, from: &str, args: &Args) -> Value {
        let (live_cwd, cwd0) = {
            let st = self.lock();
            match st.panes.get(from) {
                Some(p) if p.spawned && !st.disposed => (p.live_cwd.clone(), p.cwd.clone()),
                _ => return err("unavailable"),
            }
        };
        let preset = match args.get("agent") {
            None | Some(Value::Null) => "claude",
            Some(Value::String(s)) => s.as_str(),
            Some(_) => return err("preset"),
        };
        if !is_preset(preset) || !self.preset_enabled(preset) {
            return err("preset");
        }
        let cwd = match args.get("cwd") {
            None | Some(Value::Null) => resolve_dir(Some(live_cwd.as_deref().unwrap_or(&cwd0)), &self.home),
            Some(Value::String(s)) if Path::new(s).is_absolute() => resolve_dir(Some(s), &self.home),
            Some(_) => None,
        };
        let Some(cwd) = cwd else { return err("cwd") };
        let resume = match args.get("resume") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) if s.is_empty() => None,
            Some(v) => Some(v.clone()),
        };
        let resume = match resume {
            None => None,
            Some(Value::String(s)) if preset != "shell" && is_agent_session_id(&s) => Some(s),
            Some(_) => return err("resume"),
        };
        let now = self.now();
        let id = uuid();
        let prompt = if preset == "shell" { None } else { args.get("prompt").and_then(Value::as_str).and_then(clean_pane_prompt) };
        let submit = args.get("submit") == Some(&json!(true)) && prompt.is_some();
        let name = args.get("name").and_then(Value::as_str).filter(|n| preset == "claude" && is_pane_name(n)).map(str::to_string);
        let session_id = resume.clone().or_else(|| (preset == "claude").then(uuid));
        {
            let mut st = self.lock();
            if st.disposed {
                return err("unavailable");
            }
            let Some(p) = st.panes.get_mut(from) else { return err("unavailable") };
            p.opens.retain(|t| now - t < OPEN_RATE_MS);
            if p.opens.len() >= OPEN_RATE_MAX {
                return err("rate");
            }
            st.tickets.retain(|_, t| t.expires > now);
            if Registry::count(&mut st, now) + st.tickets.len() >= MAX_LIVE {
                return err("limit");
            }
            let fresh = resume.is_none();
            st.tickets.insert(id.clone(), Ticket { preset: preset.into(), cwd: cwd.clone(), session_id: session_id.clone().filter(|_| fresh), name: name.clone().filter(|_| fresh), expires: now + TICKET_TTL_MS });
            if let Some(p) = st.panes.get_mut(from) {
                p.opens.push(now);
            }
            st.waits.insert(id.clone(), None);
        }
        let mut req = json!({ "id": id, "from": from, "preset": preset, "cwd": cwd });
        if let Some(p) = &prompt {
            req["prompt"] = json!(p);
        }
        if let Some(r) = &resume {
            req["resume"] = json!(r);
        }
        if submit {
            req["submit"] = json!(true);
        }
        let raw = self.bridge.ask("session.open", req, from, OPEN_REPLY_MS);
        let result = to_open_result(&raw).unwrap_or_else(|| err("failed"));
        if result["ok"] == json!(true) {
            let held = resume.as_deref().and_then(|r| self.live_holding(r));
            let pane = match held {
                Some(h) => {
                    self.lock().waits.remove(&id);
                    Some(h)
                }
                None => self.wait_spawn(&id, SPAWN_WAIT_MS),
            };
            let mut out = json!({ "ok": true, "preset": preset, "cwd": cwd });
            if let Some(s) = &session_id {
                out["sessionId"] = json!(s);
            }
            if let Some(p) = pane {
                out["paneId"] = json!(p);
            }
            out["prompt"] = json!(prompt.is_some());
            if submit {
                out["submitted"] = json!(true);
            }
            if resume.is_some() {
                out["resumed"] = json!(true);
            }
            return out;
        }
        let mut st = self.lock();
        st.waits.remove(&id);
        st.tickets.remove(&id);
        if let Some(p) = st.panes.get_mut(from) {
            if let Some(at) = p.opens.iter().position(|t| *t == now) {
                p.opens.remove(at);
            }
        }
        err(result["error"].as_str().unwrap_or("failed"))
    }

    fn live_holding(&self, agent_session: &str) -> Option<String> {
        self.lock().panes.values().find(|p| p.spawned && !p.exited && p.agent_session.as_deref() == Some(agent_session)).map(|p| p.id.clone())
    }

    pub fn checkout_roots(&self, from: &str) -> Vec<String> {
        let Some(cwd) = self.lock().panes.get(from).map(|p| p.cwd.clone()) else { return vec![] };
        let res = crate::git::run_git(Path::new(&cwd), &["worktree", "list", "--porcelain"], std::time::Duration::from_secs(30));
        if !res.ok {
            return vec![];
        }
        res.stdout.lines().filter_map(|l| l.trim_end_matches('\r').strip_prefix("worktree ")).map(|p| if cfg!(windows) { p.replace('/', "\\") } else { p.to_string() }).collect()
    }
}
