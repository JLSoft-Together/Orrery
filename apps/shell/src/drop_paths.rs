use serde_json::Value;
use tauri::WebviewWindow;

const KIND: &str = "file-paths";
const MAX_FILES: u32 = 200;

pub fn request_id(text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    if v.get("ccm")?.as_str()? != KIND {
        return None;
    }
    let id = v.get("id")?.as_str()?;
    (!id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).then(|| id.to_string())
}

pub fn reply(id: &str, paths: &[String]) -> String {
    serde_json::json!({ "ccm": KIND, "id": id, "paths": paths }).to_string()
}

#[cfg(windows)]
mod platform {
    use tauri::webview::PlatformWebview;
    use tauri::{AppHandle, Manager};
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2File, ICoreWebView2WebMessageReceivedEventArgs, ICoreWebView2WebMessageReceivedEventArgs2};
    use webview2_com::{take_pwstr, WebMessageReceivedEventHandler};
    use windows::core::{Interface, HSTRING, PWSTR};

    use super::{reply, request_id, MAX_FILES};
    use crate::state::Shell;

    fn from_app(app: &AppHandle, source: &str) -> bool {
        source.parse::<tauri::Url>().is_ok_and(|u| app.state::<Shell>().is_app_url(&u))
    }

    unsafe fn paths_of(args: &ICoreWebView2WebMessageReceivedEventArgs) -> Vec<String> {
        let Ok(args) = args.cast::<ICoreWebView2WebMessageReceivedEventArgs2>() else { return Vec::new() };
        let Ok(list) = args.AdditionalObjects() else { return Vec::new() };
        let mut count = 0u32;
        if list.Count(&mut count).is_err() {
            return Vec::new();
        }
        (0..count.min(MAX_FILES))
            .map(|i| {
                let Ok(file) = list.GetValueAtIndex(i).and_then(|o| o.cast::<ICoreWebView2File>()) else { return String::new() };
                let mut raw = PWSTR::null();
                if file.Path(&mut raw).is_err() {
                    return String::new();
                }
                take_pwstr(raw)
            })
            .collect()
    }

    pub fn attach(p: &PlatformWebview, app: AppHandle) {
        unsafe {
            let Ok(core) = p.controller().CoreWebView2() else { return };
            let handler = WebMessageReceivedEventHandler::create(Box::new(move |sender, args| {
                let (Some(sender), Some(args)) = (sender, args) else { return Ok(()) };
                let mut raw = PWSTR::null();
                if args.TryGetWebMessageAsString(&mut raw).is_err() {
                    return Ok(());
                }
                let Some(id) = request_id(&take_pwstr(raw)) else { return Ok(()) };
                let mut src = PWSTR::null();
                if args.Source(&mut src).is_err() || !from_app(&app, &take_pwstr(src)) {
                    return Ok(());
                }
                let json = reply(&id, &paths_of(&args));
                let _ = sender.PostWebMessageAsJson(&HSTRING::from(json));
                Ok(())
            }));
            let mut token = 0i64;
            let _ = core.add_WebMessageReceived(&handler, &mut token);
        }
    }
}

#[cfg(windows)]
pub fn attach(win: &WebviewWindow) {
    use tauri::Manager;
    let app = win.app_handle().clone();
    let _ = win.with_webview(move |p| platform::attach(&p, app));
}

#[cfg(not(windows))]
pub fn attach(_win: &WebviewWindow) {}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

pub fn match_names(names: &[String], paths: &[String]) -> Vec<String> {
    let mut used = vec![false; paths.len()];
    let mut out: Vec<Option<usize>> = names
        .iter()
        .map(|name| {
            let hit = paths.iter().enumerate().position(|(i, p)| !used[i] && !name.is_empty() && file_name(p) == name)?;
            used[hit] = true;
            Some(hit)
        })
        .collect();
    if names.len() == paths.len() {
        for (i, slot) in out.iter_mut().enumerate() {
            if slot.is_none() && !used[i] {
                used[i] = true;
                *slot = Some(i);
            }
        }
    }
    out.into_iter().map(|i| i.map_or_else(String::new, |i| paths[i].clone())).collect()
}

#[cfg(target_os = "macos")]
fn drag_pasteboard() -> Vec<String> {
    use objc2::ClassType;
    use objc2_app_kit::{NSPasteboard, NSPasteboardNameDrag};
    use objc2_foundation::{NSArray, NSURL};

    let pb = NSPasteboard::pasteboardWithName(unsafe { NSPasteboardNameDrag });
    let classes = NSArray::from_slice(&[NSURL::class()]);
    let Some(items) = (unsafe { pb.readObjectsForClasses_options(&classes, None) }) else { return Vec::new() };
    items
        .iter()
        .take(MAX_FILES as usize)
        .filter_map(|item| {
            let url = item.downcast_ref::<NSURL>()?;
            if !url.isFileURL() {
                return None;
            }
            Some(url.filePathURL()?.path()?.to_string())
        })
        .collect()
}

#[cfg(target_os = "macos")]
pub fn for_names(app: &tauri::AppHandle, names: Vec<String>) -> Vec<String> {
    let blank = vec![String::new(); names.len()];
    if names.is_empty() || names.len() > MAX_FILES as usize {
        return blank;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let read = move || {
        let _ = tx.send(drag_pasteboard());
    };
    if app.run_on_main_thread(read).is_err() {
        return blank;
    }
    match rx.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok(paths) => match_names(&names, &paths),
        Err(_) => blank,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn for_names(_app: &tauri::AppHandle, names: Vec<String>) -> Vec<String> {
    vec![String::new(); names.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_file_path_requests() {
        assert_eq!(request_id(r#"{"ccm":"file-paths","id":"3f2a-9c"}"#).as_deref(), Some("3f2a-9c"));
        assert_eq!(request_id(r#"{"ccm":"other","id":"a"}"#), None);
        assert_eq!(request_id(r#"{"ccm":"file-paths","id":""}"#), None);
        assert_eq!(request_id(r#"{"ccm":"file-paths","id":"a\"b"}"#), None);
        assert_eq!(request_id(&format!(r#"{{"ccm":"file-paths","id":"{}"}}"#, "a".repeat(65))), None);
        assert_eq!(request_id(r#"{"cmd":"plugin:event|listen","callback":1}"#), None);
        assert_eq!(request_id("not json"), None);
    }

    #[test]
    fn reply_keeps_order_and_blanks() {
        let v: Value = serde_json::from_str(&reply("x1", &["C:\\a b\\c.txt".into(), String::new()])).unwrap();
        assert_eq!(v["ccm"], "file-paths");
        assert_eq!(v["id"], "x1");
        assert_eq!(v["paths"], serde_json::json!(["C:\\a b\\c.txt", ""]));
    }

    #[test]
    fn names_pick_their_own_path() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let paths = s(&["/Users/a/b c.png", "/tmp/x/notes.md", "/tmp/dir"]);
        assert_eq!(match_names(&s(&["notes.md", "dir"]), &paths), s(&["/tmp/x/notes.md", "/tmp/dir"]));
        assert_eq!(match_names(&s(&["missing.txt"]), &paths), s(&[""]));
        assert_eq!(match_names(&s(&["a.txt", "a.txt"]), &s(&["/p/a.txt", "/q/a.txt"])), s(&["/p/a.txt", "/q/a.txt"]));
        assert_eq!(match_names(&s(&["Tie\u{0301}ng.md"]), &s(&["/p/Tiếng.md"])), s(&["/p/Tiếng.md"]));
        assert_eq!(match_names(&s(&["x"]), &[]), s(&[""]));
    }
}
