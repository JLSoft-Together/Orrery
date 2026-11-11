use super::*;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("orrery-host-run-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("sub")).unwrap();
    PathBuf::from(real_or_null(&dir.to_string_lossy()).unwrap())
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn path_of(url: &str) -> String {
    url.split_once("://").unwrap().1.split_once('/').map(|(_, rest)| format!("/{rest}")).unwrap()
}

#[test]
fn the_run_owner_is_the_window_label_and_key() {
    assert_eq!(owner_key("main", "pane-1"), "main:pane-1");
    assert_eq!(owner_key(MAIN, ""), format!("{MAIN}:"));
    assert_eq!(text(&json!({"key": 7}), "key"), "7");
    assert_eq!(text(&json!({}), "key"), "");
}

#[test]
fn reinstall_names_the_package_in_both_languages() {
    let en = run_confirm_spec(false, "reinstall", Some("com.app")).unwrap();
    assert_eq!(en.message, "Uninstall the old build and reinstall?");
    assert!(en.detail.starts_with("The installed com.app was signed"));
    assert_eq!((en.ok, en.cancel), ("Uninstall and reinstall", "Cancel"));
    let vi = run_confirm_spec(true, "reinstall", None).unwrap();
    assert_eq!((vi.ok, vi.cancel), ("Gỡ và cài lại", "Huỷ"));
    assert!(vi.detail.starts_with(" trên máy"));
}

#[test]
fn trust_config_lists_folder_command_steps_and_env_keys() {
    let plan = json!({"name": "app", "cwd": "/p", "command": "flutter run", "beforeLaunch": ["pub get", 3], "envKeys": ["A", "B"]}).to_string();
    let en = run_confirm_spec(false, "trust-config", Some(&plan)).unwrap();
    assert_eq!(en.message, "Run \"app\" from the project files?");
    assert_eq!(en.ok, "Trust and run");
    let lines: Vec<&str> = en.detail.lines().collect();
    assert_eq!(lines[1..], ["Folder: /p", "Command: flutter run", "Before launch:", "  pub get", "Environment variables: A, B", "You are asked again whenever the configuration changes."]);
    let bare = json!({"name": "x", "cwd": "/p", "command": ""}).to_string();
    let vi = run_confirm_spec(true, "trust-config", Some(&bare)).unwrap();
    assert_eq!(vi.message, "Chạy \"x\" từ file của project?");
    assert_eq!(vi.detail.lines().collect::<Vec<_>>()[1..], ["Thư mục: /p", "Lệnh: —", "Mỗi khi cấu hình thay đổi, app sẽ hỏi lại."]);
}

#[test]
fn an_unknown_code_or_a_broken_plan_never_opens_a_dialog() {
    assert!(run_confirm_spec(false, "other", Some("x")).is_none());
    assert!(run_confirm_spec(false, "trust-config", None).is_none());
    assert!(run_confirm_spec(false, "trust-config", Some("not json")).is_none());
    assert!(run_confirm_spec(false, "trust-config", Some(&json!({"name": "x", "cwd": "/p"}).to_string())).is_none());
}

#[test]
fn the_layout_download_dialog_rounds_megabytes_and_names_the_host() {
    let en = layout_confirm_spec(false, 41_600_000.0, "");
    assert_eq!(en.message, "Download the Android layout renderer?");
    assert!(en.detail.contains("about 42 MB) from dl.google.com."));
    assert_eq!((en.ok, en.cancel), ("Download", "Cancel"));
    let vi = layout_confirm_spec(true, f64::NAN, "github.com");
    assert!(vi.detail.contains("khoảng 0 MB) từ github.com."));
    assert_eq!(vi.ok, "Tải về");
}

#[test]
fn layout_urls_are_minted_only_for_images_and_served_by_the_preview_server() {
    let dir = scratch("layout");
    fs::write(dir.join("sub/a b.png"), b"PNGDATA").unwrap();
    fs::write(dir.join("sub/notes.txt"), b"x").unwrap();
    let url = layout_url(&json!({"file": s(&dir.join("sub/a b.png")), "roots": [s(&dir)]}))["url"].as_str().unwrap().to_owned();
    assert!(url.starts_with("http://127.0.0.1:") && url.ends_with("/sub/a%20b.png"), "{url}");
    assert_eq!(layout_url(&json!({"file": s(&dir.join("sub/notes.txt"))})), Value::Null);
    assert_eq!(layout_url(&json!({"file": s(&dir.join("sub"))})), Value::Null);
    assert_eq!(layout_url(&json!({"file": "sub/a b.png"})), Value::Null);
    let port: u16 = url.split(':').nth(2).unwrap().split('/').next().unwrap().parse().unwrap();
    let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
    c.write_all(format!("GET {} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n", path_of(&url)).as_bytes()).unwrap();
    let mut out = String::new();
    c.read_to_string(&mut out).unwrap();
    assert!(out.starts_with("HTTP/1.1 200 OK\r\n") && out.contains("Content-Type: image/png\r\n") && out.ends_with("\r\n\r\nPNGDATA"), "{out}");
    let _ = fs::remove_dir_all(&dir);
}
