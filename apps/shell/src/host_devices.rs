use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use core_desk::capture::files::{is_inside, join, CAPTURE_FILE, REC_SUB};
use core_desk::capture::folder::Folder;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Webview};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder, FilePath};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use crate::hosts::{confirm, locale};
use crate::state::{lock, Shell, MAIN};

pub const USE_GAP: Duration = Duration::from_secs(15);
const TEMP_REC_DIR: &str = "orrery-rec";
const ANDROID_EXT: [&str; 2] = ["apk", "aab"];

struct Text {
    cancel: &'static str,
    rec_save_title: &'static str,
    rec_filter: &'static str,
    pull_title: &'static str,
    push_title: &'static str,
    release_pick_title: &'static str,
    capture_pick_title: &'static str,
    capture_missing_title: &'static str,
    vi: bool,
}

const EN: Text = Text {
    cancel: "Cancel",
    rec_save_title: "Save screen recording",
    rec_filter: "MP4 video",
    pull_title: "Pull to folder",
    push_title: "Push files to the device",
    release_pick_title: "Choose the APK or AAB to verify",
    capture_pick_title: "Folder for screenshots and recordings",
    capture_missing_title: "Capture folder not available",
    vi: false,
};

const VI: Text = Text {
    cancel: "Huỷ",
    rec_save_title: "Lưu video quay màn hình",
    rec_filter: "Video MP4",
    pull_title: "Kéo về thư mục",
    push_title: "Đẩy file vào thiết bị",
    release_pick_title: "Chọn APK hoặc AAB để kiểm chữ ký",
    capture_pick_title: "Thư mục lưu ảnh chụp và video quay",
    capture_missing_title: "Không dùng được thư mục chụp / quay",
    vi: true,
};

fn text(locale: &str) -> &'static Text {
    if locale == "en" {
        &EN
    } else {
        &VI
    }
}

fn clip(s: &str) -> String {
    if s.chars().count() > 300 {
        format!("{}…", s.chars().take(300).collect::<String>())
    } else {
        s.to_owned()
    }
}

fn field_text(p: &Value, key: &str) -> String {
    match p.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

pub struct Ask {
    pub message: String,
    pub detail: String,
    pub ok: String,
}

impl Text {
    fn capture_missing_body(&self, dir: &str) -> String {
        if self.vi {
            format!("Không tìm thấy {dir} (đã rút ổ?). Video quay tạm ghi vào thư mục temp và ảnh chụp đi qua clipboard cho tới khi thư mục có lại.")
        } else {
            format!("{dir} cannot be found (drive removed?). Recordings go to the temp folder and screenshots to the clipboard until it is back.")
        }
    }

    fn ask(&self, p: &Value) -> Option<Ask> {
        let t = |k: &str| field_text(p, k);
        let vi = self.vi;
        let ask = |message: String, detail: String, ok: &str| Some(Ask { message, detail, ok: ok.to_owned() });
        match p.get("kind").and_then(Value::as_str)? {
            "remove" => {
                let ids: Vec<String> = p.get("ids")?.as_array()?.iter().map(|v| v.as_str().map(str::to_owned)).collect::<Option<_>>()?;
                let n = ids.len();
                let message = match (vi, n) {
                    (true, 1) => "Quên thiết bị này?".to_owned(),
                    (true, _) => format!("Quên {n} thiết bị?"),
                    (false, 1) => "Forget this device?".to_owned(),
                    (false, _) => format!("Forget {n} devices?"),
                };
                let tail = if vi {
                    "Orrery sẽ không hiện và tự kết nối các thiết bị này nữa. Không có gì trên thiết bị bị thay đổi."
                } else {
                    "Orrery stops listing and auto-connecting them. Nothing on the devices changes."
                };
                ask(message, format!("{}\n\n{tail}", ids.join("\n")), if vi { "Quên" } else { "Forget" })
            }
            "wipe" => {
                let name = p.get("name")?.as_str()?;
                if vi {
                    ask(format!("Xoá toàn bộ dữ liệu của emulator {name}?"), "Ứng dụng, tài khoản, file và snapshot trong thiết bị ảo này sẽ bị xoá, rồi nó khởi động như mới. Không hoàn tác được.".into(), "Xoá và khởi động")
                } else {
                    ask(format!("Wipe all data of the emulator {name}?"), "Apps, accounts, files and snapshots inside this virtual device are erased, then it starts like new. This cannot be undone.".into(), "Wipe and start")
                }
            }
            "fs-delete" => {
                let (name, dir, at) = (t("name"), p.get("dir") == Some(&Value::Bool(true)), t("where"));
                if vi {
                    let m = if dir { format!("Xoá thư mục {name} cùng mọi thứ bên trong khỏi thiết bị?") } else { format!("Xoá {name} khỏi thiết bị?") };
                    ask(m, format!("{at}\n\nKhông hoàn tác được."), "Xoá")
                } else {
                    let m = if dir { format!("Delete the folder {name} and everything in it from the device?") } else { format!("Delete {name} from the device?") };
                    ask(m, format!("{at}\n\nThis cannot be undone."), "Delete")
                }
            }
            "fs-replace" => {
                let (name, at) = (t("name"), t("where"));
                if vi {
                    ask(format!("Thay {name} trên thiết bị?"), format!("{at} đã có và sẽ bị ghi đè."), "Thay")
                } else {
                    ask(format!("Replace {name} on the device?"), format!("{at} already exists and will be overwritten."), "Replace")
                }
            }
            "perf-cold" => {
                let pkg = t("pkg");
                if vi {
                    ask(format!("Đo thời gian mở nguội của {pkg}?"), "Ứng dụng sẽ bị dừng trước (trạng thái chưa lưu trong app sẽ mất), rồi activity khởi chạy được mở và thời gian mở được đọc lại.".into(), "Dừng và mở app")
                } else {
                    ask(format!("Measure a cold start of {pkg}?"), "The app is stopped first (unsaved state in it is lost), then its launcher activity is started and the start time is read.".into(), "Stop and start app")
                }
            }
            "pref-write" => {
                let (pkg, file, key) = (t("pkg"), t("file"), t("key"));
                let change = format!("{file}\n{key}: {} → {}", clip(&t("before")), clip(&t("after")));
                if vi {
                    ask(format!("Đổi một giá trị SharedPreferences của {pkg}?"), format!("{change}\n\nỨng dụng sẽ bị dừng (force-stop) trước, rồi file được ghi lên thiết bị. Bản gốc đầu tiên của file được giữ trong Orrery để Khôi phục bản gốc."), "Ghi và dừng ứng dụng")
                } else {
                    ask(format!("Change a SharedPreferences value of {pkg}?"), format!("{change}\n\nThe app is force-stopped first, then the file is written on the device. The first original of this file is kept in Orrery for Restore original."), "Write and stop app")
                }
            }
            "pref-restore" => {
                let (pkg, file) = (t("pkg"), t("file"));
                if vi {
                    ask(format!("Khôi phục file SharedPreferences gốc của {pkg}?"), format!("{file} sẽ trở về bản đã giữ trước lần đổi đầu tiên từ Orrery. Ứng dụng sẽ bị dừng (force-stop) trước."), "Khôi phục và dừng ứng dụng")
                } else {
                    ask(format!("Restore the original SharedPreferences file of {pkg}?"), format!("{file} goes back to the copy kept before the first change from Orrery. The app is force-stopped first."), "Restore and stop app")
                }
            }
            _ => None,
        }
    }
}

fn valid_serial(s: &str) -> bool {
    (1..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

fn should_report(last: &mut HashMap<String, Instant>, serial: &str, now: Instant, gap: Duration) -> bool {
    if !valid_serial(serial) {
        return false;
    }
    if last.get(serial).is_some_and(|t| now.duration_since(*t) < gap) {
        return false;
    }
    last.insert(serial.to_owned(), now);
    true
}

pub fn device_used(app: &AppHandle, serial: &str, gap: Duration) {
    static LAST: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
    if !should_report(&mut lock(LAST.get_or_init(Default::default)), serial, Instant::now(), gap) {
        return;
    }
    let at = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_millis() as f64);
    let msg = json!({"type": "device.used", "serial": serial, "at": at});
    if let Some(serve) = lock(&app.state::<Shell>().serve).as_mut() {
        serve.post(&msg);
    }
}

fn owner_key(label: &str, key: &str) -> String {
    format!("{label}:{key}")
}

fn rec_dir(active: Option<&str>) -> PathBuf {
    active.map_or_else(|| std::env::temp_dir().join(TEMP_REC_DIR), |d| PathBuf::from(join(d, REC_SUB)))
}

fn keeps(dir: Option<&str>, file: &str) -> bool {
    dir.is_some_and(|d| is_inside(&join(d, REC_SUB), file))
}

fn folder(app: &AppHandle) -> Folder {
    Folder::new(app.state::<Shell>().data_dir.join(CAPTURE_FILE))
}

fn save_dir(app: &AppHandle) -> PathBuf {
    let videos = || app.path().video_dir().unwrap_or_else(|_| std::env::temp_dir());
    match folder(app).status().active {
        Some(dir) => {
            let rec = PathBuf::from(join(&dir, REC_SUB));
            if std::fs::create_dir_all(&rec).is_ok() {
                rec
            } else {
                videos()
            }
        }
        None => videos(),
    }
}

fn e2e(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn dialog(app: &AppHandle) -> FileDialogBuilder<tauri::Wry> {
    let mut d = app.dialog().file();
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.set_parent(&w);
    }
    d
}

fn path_of(p: Option<FilePath>) -> Value {
    p.and_then(|p| p.into_path().ok()).map_or(Value::Null, |p| json!(p.to_string_lossy()))
}

fn str_param<'a>(p: &'a Value, key: &str) -> Option<&'a str> {
    p.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn pick_android(app: &AppHandle, title: Option<&str>, near: Option<&str>) -> Value {
    let mut d = dialog(app).add_filter("Android", &ANDROID_EXT);
    if let Some(t) = title {
        d = d.set_title(t);
    }
    if let Some(dir) = near {
        d = d.set_directory(dir);
    }
    path_of(d.blocking_pick_file())
}

fn rec_save(app: &AppHandle, p: &Value) -> Value {
    if let Some(f) = e2e("CCM_E2E_RECORD_FILE") {
        return json!(f);
    }
    let t = text(&locale(app));
    let name = str_param(p, "defaultName").unwrap_or("screen.mp4");
    let d = dialog(app).set_title(t.rec_save_title).add_filter(t.rec_filter, &["mp4"]).set_directory(save_dir(app)).set_file_name(name);
    path_of(d.blocking_save_file())
}

fn pick_pull_dir(app: &AppHandle) -> Value {
    if let Some(d) = e2e("CCM_E2E_PULL_DIR") {
        return json!(d);
    }
    let mut d = dialog(app).set_title(text(&locale(app)).pull_title).set_can_create_directories(true);
    if let Ok(dl) = app.path().download_dir() {
        d = d.set_directory(dl);
    }
    path_of(d.blocking_pick_folder())
}

fn pick_push_files(app: &AppHandle) -> Value {
    if let Some(f) = e2e("CCM_E2E_PUSH_FILE") {
        return json!([f]);
    }
    let picked = dialog(app).set_title(text(&locale(app)).push_title).blocking_pick_files();
    let files: Vec<String> = picked.unwrap_or_default().into_iter().filter_map(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned()).collect();
    if files.is_empty() {
        Value::Null
    } else {
        json!(files)
    }
}

fn open_local(app: &AppHandle, p: &Value) -> bool {
    let Some(local) = str_param(p, "local") else { return false };
    match p.get("mode").and_then(Value::as_str) {
        Some("reveal") => {
            let _ = app.opener().reveal_item_in_dir(local);
            true
        }
        Some("open") => app.opener().open_path(local, None::<&str>).is_ok(),
        _ => false,
    }
}

fn capture_pick(app: &AppHandle, p: &Value) -> Value {
    if let Some(d) = e2e("CCM_E2E_CAPTURE_DIR") {
        return json!(d);
    }
    let mut d = dialog(app).set_title(text(&locale(app)).capture_pick_title).set_can_create_directories(true);
    match str_param(p, "near") {
        Some(near) => d = d.set_directory(near),
        None => {
            if let Ok(pics) = app.path().picture_dir() {
                d = d.set_directory(pics);
            }
        }
    }
    path_of(d.blocking_pick_folder())
}

fn capture_missing(app: &AppHandle, p: &Value) {
    let t = text(&locale(app));
    let dir = str_param(p, "dir").unwrap_or_default();
    let _ = app.notification().builder().title(t.capture_missing_title).body(t.capture_missing_body(dir)).show();
}

fn devices_confirm(app: &AppHandle, p: &Value) -> bool {
    let t = text(&locale(app));
    match t.ask(p) {
        Some(a) => confirm(app, &a.message, &a.detail, &a.ok, t.cancel),
        None => false,
    }
}

pub fn call(app: &AppHandle, caller: Option<&Webview>, method: &str, p: &Value) -> Option<Result<Value, String>> {
    let v = match method {
        "devices.confirm" => json!(devices_confirm(app, p)),
        "devices.owner" => json!(owner_key(caller.map_or(MAIN, |w| w.label()), p.get("key").and_then(Value::as_str).unwrap_or(""))),
        "devices.use" => {
            if let Some(serial) = str_param(p, "serial") {
                device_used(app, serial, USE_GAP);
            }
            json!(true)
        }
        "devices.recDir" => json!(rec_dir(folder(app).status().active.as_deref()).to_string_lossy()),
        "devices.recKeeps" => json!(str_param(p, "file").is_some_and(|f| keeps(folder(app).dir().as_deref(), f))),
        "devices.recSave" => rec_save(app, p),
        "devfs.pickFolder" => pick_pull_dir(app),
        "devfs.pickFiles" => pick_push_files(app),
        "devfs.progress" => {
            crate::desk::emit_app(app, "ccm:devfs-progress", p.clone());
            json!(true)
        }
        "devfs.open" => json!(open_local(app, p)),
        "apk.pick" => pick_android(app, None, str_param(p, "defaultPath")),
        "release.pick" => match e2e("CCM_E2E_RELEASE_ARTIFACT") {
            Some(f) => json!(f),
            None => pick_android(app, Some(text(&locale(app)).release_pick_title), str_param(p, "near")),
        },
        "capture.pick" => capture_pick(app, p),
        "capture.open" => json!(str_param(p, "dir").is_some_and(|d| app.opener().open_path(d, None::<&str>).is_ok())),
        "capture.missing" => {
            capture_missing(app, p);
            Value::Null
        }
        "capture.held" => json!([]),
        _ => return None,
    };
    Some(Ok(v))
}

#[cfg(test)]
#[path = "host_devices_tests.rs"]
mod tests;
