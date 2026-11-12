use std::path::PathBuf;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Webview};
use tauri_plugin_dialog::DialogExt;

use crate::appearance::{is_hex, next_zoom, parse_look, zoom_percent};
use crate::shot;
use crate::state::{lock, Shell};
use crate::{desk, guest};
use crate::tray;
use crate::window::{open_external, set_background_all, set_zoom_all, window_action};

#[cfg(test)]
const LOCAL: [&str; 17] = [
    "ccm:pty-endpoint",
    "ccm:pick-folder",
    "ccm:open-external",
    "ccm:zoom",
    "ccm:window-bg",
    "ccm:appearance",
    "ccm:locale",
    "ccm:login-item",
    "ccm:orrery-reply",
    "ccm:focus-reply",
    "shell:window",
    "shell:guest",
    "ccm:capture-guest",
    "ccm:capture-rect",
    "ccm:clipboard-image",
    "ccm:clipboard-image-file",
    "ccm:drop-paths",
];

fn arg(args: &[Value], i: usize) -> &Value {
    args.get(i).unwrap_or(&Value::Null)
}

fn start_dir(near: &Value) -> Option<PathBuf> {
    let p = PathBuf::from(near.as_str()?);
    (p.is_absolute() && p.is_dir()).then_some(p)
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

fn zoom(app: &AppHandle, action: &Value) -> Value {
    let shell = app.state::<Shell>();
    let mut look = lock(&shell.look);
    let Some(action) = action.as_str() else { return Value::Null };
    if action == "get" {
        return json!(zoom_percent(look.state.zoom));
    }
    let Some(level) = next_zoom(look.state.zoom, action) else { return Value::Null };
    if level != look.state.zoom {
        look.state.zoom = level;
        look.save();
    }
    drop(look);
    set_zoom_all(app, level);
    json!(zoom_percent(level))
}

fn appearance(app: &AppHandle, raw: &Value) -> Value {
    let Some(look) = parse_look(raw) else { return json!(false) };
    let shell = app.state::<Shell>();
    let mut store = lock(&shell.look);
    if store.state.look.as_ref() != Some(&look) {
        let canvas = look.get("canvas").and_then(Value::as_str).unwrap_or("#ececee").to_owned();
        store.state.look = Some(look);
        store.state.background = Some(canvas.clone());
        store.save();
        drop(store);
        set_background_all(app, &canvas);
    }
    json!(true)
}

pub async fn dispatch(app: AppHandle, webview: Webview, channel: &str, args: Vec<Value>) -> Result<Value, String> {
    let shell = app.state::<Shell>();
    match channel {
        "ccm:pty-endpoint" => {
            let handle = app.clone();
            let ep = blocking(move || handle.state::<Shell>().pty.endpoint()).await??;
            Ok(json!({ "url": ep.url, "protocol": ep.protocol }))
        }
        "ccm:pick-folder" => {
            let near = start_dir(arg(&args, 0));
            let handle = app.clone();
            let picked = blocking(move || {
                let mut d = handle.dialog().file();
                if let Some(dir) = near {
                    d = d.set_directory(dir);
                }
                d.blocking_pick_folder()
            })
            .await?;
            let folder = picked.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned());
            if let (Some(f), Some(gate)) = (folder.as_deref(), app.try_state::<crate::preview_host::Gate>()) {
                gate.grants.add(f);
            }
            Ok(folder.map_or(Value::Null, |f| json!(f)))
        }
        "ccm:open-external" => {
            if let Some(url) = arg(&args, 0).as_str() {
                open_external(&app, url);
            }
            Ok(json!(true))
        }
        "ccm:zoom" => Ok(zoom(&app, arg(&args, 0))),
        "ccm:window-bg" => {
            let Some(color) = arg(&args, 0).as_str().filter(|c| is_hex(c)).map(str::to_ascii_lowercase) else { return Ok(json!(false)) };
            {
                let mut look = lock(&shell.look);
                look.state.background = Some(color.clone());
                look.save();
            }
            set_background_all(&app, &color);
            Ok(json!(true))
        }
        "ccm:appearance" => Ok(appearance(&app, arg(&args, 0))),
        "ccm:locale" => {
            let Some(locale) = arg(&args, 0).as_str().filter(|l| *l == "en" || *l == "vi") else { return Ok(json!(false)) };
            {
                let mut look = lock(&shell.look);
                look.state.locale = Some(locale.to_owned());
                look.save();
            }
            tray::refresh(&app);
            Ok(json!(true))
        }
        "ccm:login-item" => {
            if !tray::login_supported() {
                return Ok(Value::Null);
            }
            Ok(match arg(&args, 0) {
                Value::Bool(on) => json!(tray::set_open_at_login(&app, *on)),
                _ => json!(tray::open_at_login(&app)),
            })
        }
        "ccm:orrery-reply" => {
            if let Some(id) = arg(&args, 0).as_str() {
                shell.reply(id, arg(&args, 1).clone());
            }
            Ok(Value::Null)
        }
        "ccm:focus-reply" => {
            shell.reply("focus", json!(arg(&args, 0) == &Value::Bool(true)));
            Ok(Value::Null)
        }
        "shell:window" => {
            let window = app.get_webview_window(webview.label()).ok_or("no window")?;
            Ok(window_action(&window, arg(&args, 0).as_str().unwrap_or("")))
        }
        "shell:guest" => {
            let owner = webview.window().label().to_owned();
            let handle = app.clone();
            blocking(move || guest::op(&handle, &owner, &args)).await?
        }
        "ccm:capture-guest" => {
            let owner = webview.window().label().to_owned();
            let handle = app.clone();
            Ok(json!(blocking(move || shot::capture_guest(&handle, &owner, arg(&args, 0))).await?))
        }
        "ccm:capture-rect" => {
            let rect = arg(&args, 0).clone();
            Ok(blocking(move || shot::capture_rect(&webview, &rect).map_or(Value::Null, |png| json!(shot::base64(&png)))).await?)
        }
        "ccm:clipboard-image" => {
            let raw = arg(&args, 0).clone();
            Ok(json!(blocking(move || shot::clipboard_data_url(&raw)).await?))
        }
        "ccm:clipboard-image-file" => {
            let raw = arg(&args, 0).clone();
            Ok(json!(blocking(move || shot::clipboard_file(&raw)).await?))
        }
        "ccm:drop-paths" => {
            let names: Vec<String> = arg(&args, 0).as_array().map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_owned()).collect()).unwrap_or_default();
            let handle = app.clone();
            Ok(json!(blocking(move || crate::drop_paths::for_names(&handle, names)).await?))
        }
        c if crate::files_ipc::handles(c) => {
            let (handle, channel) = (app.clone(), c.to_owned());
            blocking(move || crate::files_ipc::dispatch(&handle, &channel, &args)).await?
        }
        c if crate::preview_ipc::handles(c) => {
            let (handle, owner, channel) = (app.clone(), webview.label().to_owned(), c.to_owned());
            blocking(move || crate::preview_ipc::dispatch(&handle, &owner, &channel, &args)).await?
        }
        _ => desk::dispatch(app, webview, channel.to_owned(), args).await,
    }
}

#[tauri::command]
pub async fn ccm(app: AppHandle, webview: Webview, channel: String, args: Option<Vec<Value>>) -> Result<Value, String> {
    let trusted = webview.url().is_ok_and(|u| app.state::<Shell>().is_app_url(&u));
    if !trusted {
        return Err("denied".into());
    }
    dispatch(app, webview, &channel, args.unwrap_or_default()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_dir_must_be_an_existing_absolute_folder() {
        assert_eq!(start_dir(&json!(std::env::temp_dir().to_string_lossy())), Some(std::env::temp_dir()));
        assert_eq!(start_dir(&json!("relative")), None);
        assert_eq!(start_dir(&json!(5)), None);
        assert_eq!(start_dir(&Value::Null), None);
    }

    #[test]
    fn the_local_channels_exist_in_the_generated_shim() {
        let shim = include_str!("shim.js");
        for channel in LOCAL.iter().filter(|c| c.starts_with("ccm:")) {
            assert!(shim.contains(&format!("\"{channel}\"")), "{channel}");
        }
    }
}
