pub mod api;
pub mod store;

use crate::{arg, keychain, util, Ctx, Desk, DeskEnv};
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use store::{Store, GH_ACCOUNTS_FILE, MAX_ACCOUNTS};

pub const MODULE: &str = "github";
const ACCESS_TTL_MS: f64 = 10.0 * 60_000.0;
const TIMEOUT: Duration = Duration::from_secs(10);

pub struct GitHub {
    file: PathBuf,
    base: String,
    access: Mutex<HashMap<String, (f64, &'static str)>>,
    lock: Mutex<()>,
}

fn token_ok(t: &str) -> bool {
    (20..=255).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn fail(error: &str, detail: Option<String>) -> Value {
    let mut m = Map::new();
    m.insert("ok".into(), json!(false));
    m.insert("error".into(), json!(error));
    if let Some(d) = detail {
        m.insert("detail".into(), json!(d));
    }
    Value::Object(m)
}

pub fn git_auth_env(token: &str) -> Value {
    let start = std::env::var("GIT_CONFIG_COUNT").ok().and_then(|v| crate::claude::js_trim(&v).parse::<i64>().ok());
    let n = start.filter(|n| *n > 0).unwrap_or(0);
    let basic = base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
    let mut m = Map::new();
    m.insert("GIT_CONFIG_COUNT".into(), json!((n + 2).to_string()));
    m.insert(format!("GIT_CONFIG_KEY_{n}"), json!("credential.https://github.com.helper"));
    m.insert(format!("GIT_CONFIG_VALUE_{n}"), json!(""));
    m.insert(format!("GIT_CONFIG_KEY_{}", n + 1), json!("http.https://github.com/.extraHeader"));
    m.insert(format!("GIT_CONFIG_VALUE_{}", n + 1), json!(format!("Authorization: Basic {basic}")));
    Value::Object(m)
}

impl GitHub {
    pub fn new(data_dir: &std::path::Path) -> Self {
        GitHub { file: data_dir.join(GH_ACCOUNTS_FILE), base: api::api_base(), access: Mutex::new(HashMap::new()), lock: Mutex::new(()) }
    }

    pub fn store(&self) -> Store {
        Store::load(&self.file)
    }

    fn forget(&self, id: &str) {
        let prefix = format!("{id}\n");
        self.access.lock().unwrap().retain(|k, _| !k.starts_with(&prefix));
    }

    fn listed(st: &Store) -> Value {
        json!({"ok": true, "accounts": st.list()})
    }

    pub fn add(&self, ctx: &Ctx, token: &Value) -> Value {
        let tok = token.as_str().map(crate::claude::js_trim).unwrap_or("");
        if !token_ok(tok) {
            return fail("invalid", None);
        }
        if !keychain::available(ctx) {
            return fail("no-keychain", None);
        }
        let Some(res) = api::get(&self.base, tok, "/user", TIMEOUT) else { return fail("network", None) };
        if res.status == 401 || res.status == 403 {
            return fail("invalid", None);
        }
        let login = res.body.get("login").and_then(Value::as_str).map(str::to_string);
        let Some(login) = login.filter(|_| res.status == 200) else { return fail("failed", Some(format!("HTTP {}", res.status))) };
        let scopes = res.scopes.map(|s| Value::Array(s.split(',').map(crate::claude::js_trim).filter(|x| !x.is_empty()).map(|x| json!(x)).collect()));
        let Some(sealed) = keychain::seal(ctx, tok) else { return fail("no-keychain", None) };
        let _guard = self.lock.lock().unwrap();
        let mut st = self.store();
        let same = st.accounts.iter().find(|a| a.get("login").and_then(Value::as_str).is_some_and(|l| l.to_lowercase() == login.to_lowercase())).map(|a| Store::id_of(a).to_string());
        if same.is_none() && st.accounts.len() >= MAX_ACCOUNTS {
            return fail("failed", Some(format!("max {MAX_ACCOUNTS}")));
        }
        let name = res.body.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).map(|n| json!(n)).unwrap_or(Value::Null);
        let mut entry = Map::new();
        let id = same.clone().unwrap_or_else(util::uuid);
        entry.insert("id".into(), json!(id));
        entry.insert("login".into(), json!(login));
        entry.insert("name".into(), name);
        entry.insert("scopes".into(), scopes.unwrap_or(Value::Null));
        entry.insert("addedAt".into(), json!(util::now_ms() as i64));
        entry.insert("token".into(), json!(sealed));
        st.put(entry, same.as_deref());
        self.forget(&id);
        st.save();
        Self::listed(&st)
    }

    pub fn remove(&self, ctx: &Ctx, id: &Value) -> Value {
        let st = self.store();
        let Some(account) = id.as_str().and_then(|i| st.find(i)).cloned() else { return fail("not-found", None) };
        let login = account.get("login").and_then(Value::as_str).unwrap_or("").to_string();
        let yes = matches!(ctx.host("github.confirmRemove", json!({"login": login})), Ok(Value::Bool(true)));
        if !yes {
            return fail("cancelled", None);
        }
        let _guard = self.lock.lock().unwrap();
        let mut st = self.store();
        let id = Store::id_of(&account).to_string();
        if st.has(&id) {
            st.remove(&id);
            self.forget(&id);
            st.save();
        }
        Self::listed(&st)
    }

    pub fn set_default(&self, id: &Value) -> Value {
        let _guard = self.lock.lock().unwrap();
        let mut st = self.store();
        match id.as_str().filter(|i| st.has(i)) {
            Some(i) => {
                st.default_id = Some(i.to_string());
                st.save();
                Self::listed(&st)
            }
            None => fail("not-found", None),
        }
    }

    pub fn set_choice(&self, root: &str, choice: &Value) -> bool {
        let _guard = self.lock.lock().unwrap();
        let mut st = self.store();
        if !st.set_choice(root, choice) {
            return false;
        }
        st.save();
        true
    }

    fn token(ctx: &Ctx, st: &Store, id: &str) -> Option<String> {
        let sealed = st.find(id)?.get("token")?.as_str()?.to_string();
        keychain::unseal(ctx, &sealed)
    }

    pub fn access_of(&self, ctx: &Ctx, st: &Store, id: &str, owner: &str, repo: &str) -> &'static str {
        let key = format!("{id}\n{owner}/{repo}").to_lowercase();
        let now = util::now_ms();
        if let Some((at, v)) = self.access.lock().unwrap().get(&key) {
            if now - at < ACCESS_TTL_MS {
                return v;
            }
        }
        let Some(token) = Self::token(ctx, st, id) else { return "unknown" };
        let route = format!("/repos/{}/{}", api::encode_component(owner), api::encode_component(repo));
        let value = match api::get(&self.base, &token, &route, TIMEOUT) {
            None => "unknown",
            Some(r) if r.status == 200 => {
                if r.body.get("permissions").and_then(|p| p.get("push")) == Some(&Value::Bool(true)) {
                    "push"
                } else {
                    "read"
                }
            }
            Some(r) if matches!(r.status, 401 | 403 | 404) => "none",
            Some(_) => "unknown",
        };
        if value != "unknown" {
            self.access.lock().unwrap().insert(key, (util::now_ms(), value));
        }
        value
    }

    pub fn resolve(&self, ctx: &Ctx, root: &str, remote: &str) -> Value {
        let Some((owner, repo, true)) = api::github_repo_of(remote) else { return json!({"github": false}) };
        let st = self.store();
        let choice = st.choice_of(root);
        let view = |id: Option<&str>, access: &str| {
            let account = id.and_then(|i| st.find(i)).map(|a| json!({"id": Store::id_of(a), "login": a.get("login").cloned().unwrap_or(Value::Null)})).unwrap_or(Value::Null);
            json!({"github": true, "owner": owner, "repo": repo, "choice": choice, "account": account, "access": access})
        };
        if choice == "system" {
            return view(None, "unknown");
        }
        if choice != "auto" {
            return view(Some(&choice), self.access_of(ctx, &st, &choice, &owner, &repo));
        }
        let mut ordered: Vec<String> = st.accounts.iter().map(|a| Store::id_of(a).to_string()).collect();
        let default = st.default_id.clone();
        ordered.sort_by_key(|id| std::cmp::Reverse(Some(id) == default.as_ref()));
        if ordered.is_empty() {
            return view(None, "unknown");
        }
        let found: Vec<(String, &str)> = ordered.iter().map(|id| (id.clone(), self.access_of(ctx, &st, id, &owner, &repo))).collect();
        let best = found.iter().find(|f| f.1 == "push").or_else(|| found.iter().find(|f| f.1 == "read"));
        if let Some((id, access)) = best {
            return view(Some(id), access);
        }
        if found.iter().all(|f| f.1 == "unknown") {
            view(Some(&ordered[0]), "unknown")
        } else {
            view(None, "none")
        }
    }

    pub fn env_for(&self, ctx: &Ctx, root: &str, remote: &str) -> Value {
        let r = self.resolve(ctx, root, remote);
        let Some(id) = r.get("account").and_then(|a| a.get("id")).and_then(Value::as_str) else { return Value::Null };
        match Self::token(ctx, &self.store(), id) {
            Some(t) => git_auth_env(&t),
            None => Value::Null,
        }
    }
}

fn field<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let gh = Arc::new(GitHub::new(&env.data_dir));
    let g = gh.clone();
    desk.on(MODULE, "ccm:gh-accounts", move |_, _| Ok(GitHub::listed(&g.store())));
    let g = gh.clone();
    desk.on(MODULE, "ccm:gh-account-add", move |ctx, args| Ok(g.add(ctx, arg(args, 0))));
    let g = gh.clone();
    desk.on(MODULE, "ccm:gh-account-remove", move |ctx, args| Ok(g.remove(ctx, arg(args, 0))));
    let g = gh.clone();
    desk.on(MODULE, "ccm:gh-account-default", move |_, args| Ok(g.set_default(arg(args, 0))));
    let g = gh.clone();
    desk.on(MODULE, "desk:gh-list", move |_, _| Ok(g.store().list()));
    let g = gh.clone();
    desk.on(MODULE, "desk:gh-resolve", move |ctx, args| Ok(g.resolve(ctx, field(arg(args, 0), "repo"), field(arg(args, 0), "url"))));
    let g = gh.clone();
    desk.on(MODULE, "desk:gh-env", move |ctx, args| Ok(g.env_for(ctx, field(arg(args, 0), "repo"), field(arg(args, 0), "url"))));
    desk.on(MODULE, "desk:gh-set-choice", move |_, args| {
        let p = arg(args, 0);
        Ok(json!(gh.set_choice(field(p, "repo"), p.get("choice").unwrap_or(&Value::Null))))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_env_and_tokens() {
        let env = git_auth_env("abc");
        let text = crate::js::stringify(&env);
        assert!(text.starts_with("{\"GIT_CONFIG_COUNT\":"));
        assert!(text.contains("Authorization: Basic eC1hY2Nlc3MtdG9rZW46YWJj"));
        assert!(token_ok("ghp_0123456789abcdefghij"));
        assert!(!token_ok("short"));
        assert!(!token_ok("ghp_0123456789abcdefghij!"));
    }
}
