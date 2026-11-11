use std::sync::Arc;

use core_desk::power::awake::Power;
use core_serve::StoreTap;
use serde_json::{json, Map, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use super::text::remote_text;
use super::{Host, Options, Remote, Welcome, CHANGED};
use crate::state::{lock, Shell};

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
}

pub fn install(app: &AppHandle) {
    let dir = app.state::<Shell>().data_dir.join("remote");
    let host = Arc::new(AppHost { app: app.clone(), power: core_desk::power::native::power() });
    let secrets = Box::new(super::keys::OsKeyring { service: crate::legacy::keyring_service() });
    let remote = Remote::new(host, secrets, dir, Options { mdns: true, ..Options::default() });
    let wanted = remote.wants_on();
    app.manage(remote);
    if wanted {
        let handle = app.clone();
        let _ = std::thread::Builder::new().name("remote-boot".into()).spawn(move || {
            handle.state::<Remote>().enable(true);
        });
    }
}

pub fn dispatch(app: &AppHandle, channel: &str, args: &[Value]) -> Result<Value, String> {
    app.try_state::<Remote>().ok_or("remote unavailable")?.call(channel, args)
}

pub fn shutdown(app: &AppHandle) {
    if let Some(r) = app.try_state::<Remote>() {
        r.shutdown();
    }
}
