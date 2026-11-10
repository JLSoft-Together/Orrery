use std::path::{Path, PathBuf};

use crate::csp::{cache_control_for, content_security_policy};
use crate::http::{Body, Response};
use crate::query::percent_decode;

const NOT_BUILT: &str = "Web UI not built. Run: npm run build";

pub fn mime_of(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()).map(|e| format!(".{e}")).as_deref() {
        Some(".html") => "text/html; charset=utf-8",
        Some(".js") => "text/javascript; charset=utf-8",
        Some(".css") => "text/css; charset=utf-8",
        Some(".json") => "application/json; charset=utf-8",
        Some(".svg") => "image/svg+xml",
        Some(".png") => "image/png",
        Some(".ico") => "image/x-icon",
        Some(".woff2") => "font/woff2",
        Some(".webmanifest") => "application/manifest+json",
        _ => "application/octet-stream",
    }
}

fn is_dot(segment: &str, double: bool) -> bool {
    let lower = segment.to_ascii_lowercase();
    if double {
        matches!(lower.as_str(), ".." | ".%2e" | "%2e." | "%2e%2e")
    } else {
        matches!(lower.as_str(), "." | "%2e")
    }
}

pub fn url_pathname(raw: &str) -> String {
    let unified = raw.replace('\\', "/");
    let mut stack: Vec<&str> = Vec::new();
    let segments: Vec<&str> = unified.split('/').skip(1).collect();
    let last = segments.len().saturating_sub(1);
    let mut trailing_slash = false;
    for (i, segment) in segments.iter().enumerate() {
        if is_dot(segment, true) {
            stack.pop();
            trailing_slash = i == last;
        } else if is_dot(segment, false) {
            trailing_slash = i == last;
        } else {
            stack.push(segment);
            trailing_slash = false;
        }
    }
    let mut out = String::from("/");
    out.push_str(&stack.join("/"));
    if trailing_slash && !stack.is_empty() {
        out.push('/');
    }
    out
}

pub enum Resolved {
    Bad,
    Outside,
    NotFound,
    File(PathBuf),
}

pub fn resolve(root: &Path, decoded: &str) -> Resolved {
    if decoded.contains('\0') {
        return Resolved::Bad;
    }
    let mut parts: Vec<&str> = Vec::new();
    for segment in decoded.split(|c| c == '/' || (cfg!(windows) && c == '\\')) {
        match segment {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Resolved::Outside;
                }
            }
            other => parts.push(other),
        }
    }
    if cfg!(windows) && parts.iter().any(|p| p.contains(':')) {
        return Resolved::NotFound;
    }
    let mut file = root.to_path_buf();
    for part in parts {
        file.push(part);
    }
    Resolved::File(file)
}

async fn is_dir(path: &Path) -> Option<bool> {
    tokio::fs::metadata(path).await.ok().map(|m| m.is_dir())
}

pub async fn serve(web_dist: &Path, target_path: &str) -> Response {
    let root = std::path::absolute(web_dist).unwrap_or_else(|_| web_dist.to_path_buf());
    let Some(decoded) = percent_decode(&url_pathname(target_path), false) else { return Response::empty(400) };
    let mut file = match resolve(&root, &decoded) {
        Resolved::Bad => return Response::empty(400),
        Resolved::Outside => return Response::empty(403),
        Resolved::NotFound => root.join("index.html"),
        Resolved::File(f) => f,
    };
    match is_dir(&file).await {
        Some(true) => file.push("index.html"),
        Some(false) => {}
        None => file = root.join("index.html"),
    }
    let Ok(meta) = tokio::fs::metadata(&file).await else { return Response::text(404, "text/plain", NOT_BUILT) };
    if file.extension().and_then(|e| e.to_str()) == Some("html") {
        let Ok(bytes) = tokio::fs::read(&file).await else { return Response::empty(404) };
        let html = String::from_utf8_lossy(&bytes).into_owned();
        let csp = content_security_policy(&html);
        return Response::text(200, "text/html; charset=utf-8", html)
            .header("cache-control", "no-cache")
            .header("x-content-type-options", "nosniff")
            .header("content-security-policy", &csp);
    }
    let Ok(open) = tokio::fs::File::open(&file).await else { return Response::empty(500) };
    Response { status: 200, headers: Vec::new(), body: Body::File(open, meta.len()) }
        .header("content-type", mime_of(&file))
        .header("cache-control", cache_control_for(&root, &file))
        .header("x-content-type-options", "nosniff")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pathname_collapses_dot_segments_like_the_url_parser() {
        assert_eq!(url_pathname("/a/../b"), "/b");
        assert_eq!(url_pathname("/../../etc"), "/etc");
        assert_eq!(url_pathname("/a/./b/"), "/a/b/");
        assert_eq!(url_pathname("/a/%2E%2e/b"), "/b");
        assert_eq!(url_pathname("/a\\..\\b"), "/b");
        assert_eq!(url_pathname("/"), "/");
        assert_eq!(url_pathname("/a/.."), "/");
        assert_eq!(url_pathname("/..%2fsecret"), "/..%2fsecret");
    }

    #[test]
    fn resolve_stays_inside_the_root() {
        let root = Path::new("root");
        assert!(matches!(resolve(root, "/a/b.js"), Resolved::File(f) if f == root.join("a").join("b.js")));
        assert!(matches!(resolve(root, "/"), Resolved::File(f) if f == root));
        assert!(matches!(resolve(root, "/../secret"), Resolved::Outside));
        assert!(matches!(resolve(root, "/a/../../secret"), Resolved::Outside));
        assert!(matches!(resolve(root, "/a/../b"), Resolved::File(f) if f == root.join("b")));
        assert!(matches!(resolve(root, "/a\0b"), Resolved::Bad));
        if cfg!(windows) {
            assert!(matches!(resolve(root, "/C:/x"), Resolved::NotFound));
            assert!(matches!(resolve(root, "/a\\..\\..\\x"), Resolved::Outside));
        }
    }

    #[test]
    fn mime_types_are_case_sensitive_like_path_extname() {
        assert_eq!(mime_of(Path::new("a.js")), "text/javascript; charset=utf-8");
        assert_eq!(mime_of(Path::new("a.woff2")), "font/woff2");
        assert_eq!(mime_of(Path::new("a.JS")), "application/octet-stream");
        assert_eq!(mime_of(Path::new("a")), "application/octet-stream");
    }
}
