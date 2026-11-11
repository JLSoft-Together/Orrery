use serde_json::Value;

pub struct Confirm {
    pub message: String,
    pub detail: String,
    pub ok: &'static str,
    pub cancel: &'static str,
}

fn text(req: &Value, key: &str) -> String {
    match req.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn lines(req: &Value, key: &str) -> Vec<String> {
    req.get(key).and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned)).collect()).unwrap_or_default()
}

fn count(req: &Value, key: &str) -> i64 {
    match req.get(key) {
        Some(Value::Array(a)) => a.len() as i64,
        Some(v) => v.as_f64().filter(|n| n.is_finite()).map_or(0, |n| n as i64),
        None => 0,
    }
}

fn plural(n: i64, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

fn run_title(vi: bool, kind: &str, a: &str, b: &str) -> String {
    match (vi, kind) {
        (false, "amend") => "Amend the last commit?".into(),
        (false, "ff") => format!("Fast-forward {b} to {a}?"),
        (false, "squash") => format!("Squash {a} into one commit on {b}?"),
        (false, "abort") => format!("Abort the {a} in progress?"),
        (false, "take") => format!("Take {a} from {b} into this checkout?"),
        (false, _) => format!("Restore {a} from the checkpoint?"),
        (true, "amend") => "Sửa commit cuối?".into(),
        (true, "ff") => format!("Fast-forward {b} lên {a}?"),
        (true, "squash") => format!("Gộp {a} thành một commit trên {b}?"),
        (true, "abort") => format!("Huỷ {a} đang làm dở?"),
        (true, "take") => format!("Lấy {a} từ {b} vào checkout này?"),
        (true, _) => format!("Khôi phục {a} từ checkpoint?"),
    }
}

fn run_detail(vi: bool, kind: &str, lines: &[String]) -> String {
    let list = lines.join("\n");
    match (vi, kind) {
        (false, "restore") => format!("These go back to how they were at the checkpoint; files that did not exist then go to the trash:\n\n{list}"),
        (false, "abort") => format!("Orrery will run:\n\n{list}\n\nConflict resolutions not yet committed are lost."),
        (false, "take") => format!("Orrery will run:\n\n{list}\n\nThe file in this checkout is replaced (it has no uncommitted changes)."),
        (false, _) => format!("Orrery will run:\n\n{list}"),
        (true, "restore") => format!("Các file này trở về như lúc checkpoint; file chưa có lúc đó sẽ vào thùng rác:\n\n{list}"),
        (true, "abort") => format!("Orrery sẽ chạy:\n\n{list}\n\nPhần giải xung đột chưa commit sẽ mất."),
        (true, "take") => format!("Orrery sẽ chạy:\n\n{list}\n\nFile ở checkout này bị thay (nó không có thay đổi chưa commit)."),
        (true, _) => format!("Orrery sẽ chạy:\n\n{list}"),
    }
}

fn deny_texts(vi: bool, req: &Value) -> (String, String) {
    let n = count(req, "added");
    let scope = text(req, "scope");
    let file = text(req, "file");
    let diff = text(req, "diff");
    let base = file.rsplit(['/', '\\']).next().unwrap_or("").to_owned();
    if vi {
        let what = if n == 1 { "việc này".to_owned() } else { format!("{n} mẫu này") };
        let place = if scope == "user" { "ở mọi dự án" } else { "trong dự án này" };
        let note = match scope.as_str() {
            "project" => " (chia sẻ với mọi người clone repo)",
            "local" => " (chỉ trên máy này)",
            _ => " (cài đặt người dùng, mọi dự án)",
        };
        (
            format!("Chặn {what} lần sau {place}?"),
            format!("Orrery sẽ thêm luật deny vào {file}{note}. Các khoá khác giữ nguyên; tệp cũ được giữ lại thành {base}.orrery-bak:\n\n{diff}\n\nClaude Code sẽ từ chối các lệnh tool khớp luật thay vì hỏi."),
        )
    } else {
        let what = if n == 1 { "this".to_owned() } else { format!("these {n}") };
        let place = if scope == "user" { "in every project" } else { "in this project" };
        let note = match scope.as_str() {
            "project" => " (shared with everyone who clones the repo)",
            "local" => " (only on this machine)",
            _ => " (your user settings, every project)",
        };
        (
            format!("Block {what} next time {place}?"),
            format!("Orrery will add deny rules to {file}{note}. Other keys stay; the old file is kept as {base}.orrery-bak:\n\n{diff}\n\nClaude Code will refuse matching tool calls instead of asking."),
        )
    }
}

pub fn confirm_spec(locale: &str, kind: &str, req: &Value) -> Option<Confirm> {
    let vi = locale != "en";
    let cancel = if vi { "Huỷ" } else { "Cancel" };
    let spec = |message: String, detail: String, ok: &'static str| Some(Confirm { message, detail, ok, cancel });
    match kind {
        "git" => {
            let title = text(req, "title");
            let commands = lines(req, "commands").join("\n");
            let force = req.get("force") == Some(&Value::Bool(true));
            if vi {
                let message = if force { format!("{title} chưa được merge hết. Vẫn xoá?") } else { format!("Xoá nhánh {title}?") };
                spec(message, format!("Orrery sẽ chạy:\n\n{commands}\n\nCác commit chỉ có trên nhánh này có thể bị mất."), "Xoá")
            } else {
                let message = if force { format!("{title} is not fully merged. Delete it anyway?") } else { format!("Delete branch {title}?") };
                spec(message, format!("Orrery will run:\n\n{commands}\n\nCommits only on this branch can be lost."), "Delete")
            }
        }
        "run" => {
            let k = text(req, "kind");
            spec(run_title(vi, &k, &text(req, "a"), &text(req, "b")), run_detail(vi, &k, &lines(req, "lines")), if vi { "Tiếp tục" } else { "Continue" })
        }
        "replace" => {
            let (n, files) = (count(req, "lines"), count(req, "files"));
            let paths = lines(req, "paths").join("\n");
            let (find, with) = (text(req, "find"), text(req, "replacement"));
            if vi {
                spec(
                    format!("Thay thế trong {n} dòng ở {files} file?"),
                    format!("\"{find}\" → \"{with}\"\n\n{paths}\n\nFile đã đổi kể từ lúc tìm sẽ được bỏ qua. Nội dung cũ được giữ trong Local History."),
                    "Thay thế",
                )
            } else {
                spec(
                    format!("Replace in {} across {}?", plural(n, "line", "lines"), plural(files, "file", "files")),
                    format!("\"{find}\" → \"{with}\"\n\n{paths}\n\nFiles changed since the search are skipped. The text before is kept in Local History."),
                    "Replace",
                )
            }
        }
        "perm" => {
            let n = count(req, "added");
            let (file, diff) = (text(req, "file"), text(req, "diff"));
            if vi {
                spec(
                    format!("Thêm {n} luật allow vào settings.local.json?"),
                    format!("Orrery sẽ ghi {file} (chỉ trên máy này, không chia sẻ). Các khoá khác giữ nguyên; tệp cũ được giữ lại thành settings.local.json.ccm-backup:\n\n{diff}\n\nClaude Code sẽ chạy các lệnh này không hỏi lại trong dự án này."),
                    "Thêm luật",
                )
            } else {
                spec(
                    format!("Add {} to settings.local.json?", plural(n, "allow rule", "allow rules")),
                    format!("Orrery will write {file} (only on this machine, not shared). Other keys stay; the old file is kept as settings.local.json.ccm-backup:\n\n{diff}\n\nClaude Code will run these without asking in this project."),
                    "Add rules",
                )
            }
        }
        "deny" => {
            let (message, detail) = deny_texts(vi, req);
            spec(message, detail, if vi { "Chặn" } else { "Block" })
        }
        "http" => {
            let (host, method) = (text(req, "host"), text(req, "method"));
            if vi {
                spec(
                    format!("Gửi request tới {host}?"),
                    format!("Request {method} này từ file .http sẽ đi ra internet, kèm header và body như đang hiện (gồm cả giá trị biến). Orrery hỏi một lần cho mỗi host tới khi khởi động lại ứng dụng."),
                    "Gửi",
                )
            } else {
                spec(
                    format!("Send a request to {host}?"),
                    format!("This {method} request from a .http file goes out to the internet, with the headers and body it shows (variable values included). Orrery asks once per host until the app restarts."),
                    "Send",
                )
            }
        }
        _ => None,
    }
}

pub fn access_spec(locale: &str, folder: &str) -> Confirm {
    if locale == "en" {
        Confirm {
            message: format!("Allow Orrery to open files in {folder}?"),
            detail: "The link points outside the folders of your open terminals. Allowing lets the browser and file tabs read this folder until the app quits.".into(),
            ok: "Allow",
            cancel: "Cancel",
        }
    } else {
        Confirm {
            message: format!("Cho phép Orrery mở file trong {folder}?"),
            detail: "Liên kết trỏ ra ngoài thư mục của các terminal đang mở. Cho phép thì tab trình duyệt và tab file đọc được thư mục này tới khi thoát app.".into(),
            ok: "Cho phép",
            cancel: "Huỷ",
        }
    }
}

pub fn full_access_spec(locale: &str) -> Confirm {
    if locale == "en" {
        Confirm {
            message: "Let Orrery open files in every folder?".into(),
            detail: "File tabs, Explorer, Changes and the browser tabs inside the app can read and, when you save, rename or delete, change any folder on this computer without asking, including ones with keys or .env files. Turn it off again in Settings → File access.".into(),
            ok: "Allow every folder",
            cancel: "Cancel",
        }
    } else {
        Confirm {
            message: "Cho phép Orrery mở tệp ở mọi thư mục?".into(),
            detail: "Tab file, Explorer, Changes và các tab trình duyệt trong app sẽ đọc được, và khi bạn lưu, đổi tên hay xoá thì sửa được, mọi thư mục trên máy mà không hỏi, kể cả thư mục chứa key hay file .env. Tắt lại trong Cài đặt → Quyền truy cập tệp.".into(),
            ok: "Cho phép mọi thư mục",
            cancel: "Huỷ",
        }
    }
}

pub fn grant_title(locale: &str) -> &'static str {
    if locale == "en" {
        "Choose your projects folder"
    } else {
        "Chọn thư mục chứa các project"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn access_texts_match_the_electron_strings() {
        assert_eq!(access_spec("en", "/x").message, "Allow Orrery to open files in /x?");
        assert_eq!(access_spec("vi", "/x").ok, "Cho phép");
        assert_eq!(full_access_spec("en").ok, "Allow every folder");
        assert_eq!(grant_title("vi"), "Chọn thư mục chứa các project");
    }

    #[test]
    fn git_and_run_texts_match_the_electron_strings() {
        let g = confirm_spec("en", "git", &json!({"title": "feat", "commands": ["git branch -d feat"]})).unwrap();
        assert_eq!(g.message, "Delete branch feat?");
        assert_eq!(g.detail, "Orrery will run:\n\ngit branch -d feat\n\nCommits only on this branch can be lost.");
        assert_eq!((g.ok, g.cancel), ("Delete", "Cancel"));
        let f = confirm_spec("vi", "git", &json!({"title": "feat", "commands": [], "force": true})).unwrap();
        assert_eq!(f.message, "feat chưa được merge hết. Vẫn xoá?");
        let r = confirm_spec("en", "run", &json!({"kind": "ff", "a": "main", "b": "dev", "lines": ["x"]})).unwrap();
        assert_eq!(r.message, "Fast-forward dev to main?");
        assert_eq!(r.ok, "Continue");
        assert_eq!(confirm_spec("en", "run", &json!({"kind": "amend", "lines": ["git commit --amend"]})).unwrap().message, "Amend the last commit?");
    }

    #[test]
    fn replace_perm_deny_and_http_texts_match_the_electron_strings() {
        let r = confirm_spec("en", "replace", &json!({"find": "a", "replacement": "b", "lines": 1, "files": 3, "paths": ["/x"]})).unwrap();
        assert_eq!(r.message, "Replace in 1 line across 3 files?");
        let p = confirm_spec("en", "perm", &json!({"file": "/p/.claude/settings.local.json", "added": ["Bash(ls)", "Read"], "diff": "+"})).unwrap();
        assert_eq!(p.message, "Add 2 allow rules to settings.local.json?");
        let d = confirm_spec("en", "deny", &json!({"file": "/p/.claude/settings.json", "scope": "project", "added": ["x"], "diff": "+"})).unwrap();
        assert_eq!(d.message, "Block this next time in this project?");
        assert!(d.detail.contains("(shared with everyone who clones the repo). Other keys stay; the old file is kept as settings.json.orrery-bak:"));
        let du = confirm_spec("vi", "deny", &json!({"file": "C:\\u\\settings.json", "scope": "user", "added": ["x", "y"], "diff": ""})).unwrap();
        assert_eq!(du.message, "Chặn 2 mẫu này lần sau ở mọi dự án?");
        assert!(du.detail.contains("thành settings.json.orrery-bak"));
        assert_eq!(confirm_spec("en", "http", &json!({"host": "api.x", "method": "POST"})).unwrap().message, "Send a request to api.x?");
        assert!(confirm_spec("en", "rm", &json!({})).is_none());
    }
}
