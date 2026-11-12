use std::sync::Arc;

use core_desk::power::awake::Power;
use core_serve::StoreTap;
use serde_json::{json, Map, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use super::text::remote_text;
use super::{Asked, Host, Options, PtyEndpoint, Remote, Welcome, CHANGED};
use crate::state::{lock, Shell, MAIN};

const ASK_WAIT: std::time::Duration = std::time::Duration::from_secs(4);

pub struct AppHost {
    app: AppHandle,
    power: Box<dyn Power>,
}

pub fn theme(look: Option<&Value>) -> Value {
    let Some(obj) = look.and_then(Value::as_object) else { return json!({ "mode": "light", "palette": {} }) };
    let mode = if obj.get("dark") == Some(&Value::Bool(true)) { "dark" } else { "light" };
    let palette: Map<String, Value> = obj.iter().filter(|(k, v)| k.as_str() != "dark" && v.is_string()).map(|(k, v)| (k.clone(), v.clone())).collect();
    json!({ "mode": mode, "palette": palette })
}

impl Host for AppHost {
    fn confirm_pair(&self, name: &str) -> bool {
        if !cfg!(debug_assertions) && std::env::var_os("CCM_E2E_CONFIRM").is_some() {
            return false;
        }
        let t = remote_text(&crate::hosts::locale(&self.app));
        crate::hosts::confirm(&self.app, &t.pair_title(name), t.pair_detail, t.allow, t.cancel)
    }

    fn notify(&self, title: &str, body: &str) {
        let _ = self.app.notification().builder().title(title).body(body).show();
    }

    fn tap(&self) -> Option<StoreTap> {
        lock(&self.app.state::<Shell>().serve).as_ref().and_then(crate::serve::Serve::tap)
    }

    fn welcome(&self) -> Welcome {
        let shell = self.app.state::<Shell>();
        let look = lock(&shell.look);
        Welcome { version: crate::config::APP_VERSION.to_owned(), theme: theme(look.state.look.as_ref()), locale: look.locale().to_owned() }
    }

    fn changed(&self, status: Value) {
        crate::desk::emit_app(&self.app, CHANGED, status);
    }

    fn awake(&self, on: bool) {
        if on {
            self.power.start();
        } else {
            self.power.stop();
        }
    }

    fn ask(&self, action: &str, args: Value) -> Asked {
        if self.app.get_webview_window(MAIN).is_none() {
            return Asked::Offline;
        }
        let id = crate::config::uuid();
        let req = json!({ "id": id, "action": action, "args": args, "caller": { "source": "phone" } });
        match self.app.state::<Shell>().ask(&self.app, &id, "ccm:orrery", req, ASK_WAIT) {
            Some(v) => Asked::Reply(v),
            None => Asked::Offline,
        }
    }

    fn pty(&self) -> Option<PtyEndpoint> {
        let ep = self.app.state::<Shell>().pty.endpoint().ok()?;
        Some(PtyEndpoint { url: ep.url, protocol: ep.protocol })
    }

    fn viewer(&self, owner: &str, channel: &str, args: Value, accept: &[String]) -> Value {
        let guard = super::scope::enter(owner, accept);
        let list = args.as_array().map_or(&[][..], Vec::as_slice);
        let raw = if crate::preview_ipc::handles(channel) {
            crate::preview_ipc::dispatch(&self.app, owner, channel, list)
        } else {
            crate::desk::call_inside(&self.app, channel, args.clone())
        };
        if let Some(need) = guard.finish() {
            return json!({ "needsConfirm": need });
        }
        match raw {
            Ok(v) => json!({ "raw": v }),
            Err(_) => json!({ "error": "internal" }),
        }
    }

    fn viewer_release(&self, owner: &str) {
        crate::preview_ipc::release(&self.app, owner);
    }
}

pub fn install(app: &AppHandle) {
    let dir = app.state::<Shell>().data_dir.join("remote");
    let host = Arc::new(AppHost { app: app.clone(), power: core_desk::power::native::power() });
    let secrets = Box::new(super::keys::OsKeyring { service: crate::legacy::keyring_service() });
    let remote = Remote::new(host, secrets, dir, Options { mdns: true, ..Options::default() });
    let wanted = remote.wants_on();
    {
        let handle = app.clone();
        remote.set_delete_confirm(Box::new(move || {
            let vi = crate::hosts::locale(&handle).starts_with("vi");
            let (title, detail, ok, cancel) = if vi {
                ("Xoá relay trên Cloudflare?", "Worker orrery-relay sẽ bị xoá khỏi tài khoản Cloudflare của bạn. Điện thoại đã ghép chỉ còn nối được qua LAN.", "Xoá Worker", "Huỷ")
            } else {
                ("Delete the relay on Cloudflare?", "The orrery-relay Worker will be deleted from your Cloudflare account. Paired phones can then only connect over LAN.", "Delete Worker", "Cancel")
            };
            crate::hosts::confirm(&handle, title, detail, ok, cancel)
        }));
    }
    app.manage(remote);
    if wanted {
        let handle = app.clone();
        let _ = std::thread::Builder::new().name("remote-boot".into()).spawn(move || {
            handle.state::<Remote>().enable(true);
        });
    }
}

pub fn shutdown(app: &AppHandle) {
    if let Some(r) = app.try_state::<Remote>() {
        r.shutdown();
    }
}

pub fn is_viewer_owner(owner: &str) -> bool {
    owner.starts_with("remote:")
}

pub fn ask_device(app: &AppHandle, owner: &str, needs: &Value) -> bool {
    app.try_state::<Remote>().is_some_and(|r| r.ask_device(owner, needs))
}

pub fn viewer_event(app: &AppHandle, owner: &str, event: super::VEvent) {
    if let Some(r) = app.try_state::<Remote>() {
        r.viewer_event(owner, event);
    }
}

pub fn run_event(app: &AppHandle, payload: &Value) {
    if let Some(r) = app.try_state::<Remote>() {
        r.run_event(payload);
    }
}
