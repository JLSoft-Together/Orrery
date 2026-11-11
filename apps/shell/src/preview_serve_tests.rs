use super::*;
use std::fs;

fn scratch(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("orrery-preview-serve-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("sub")).unwrap();
    real_or_null(&dir.to_string_lossy()).unwrap()
}

fn join(dir: &str, rel: &str) -> String {
    Path::new(dir).join(rel).to_string_lossy().into_owned()
}

fn get(s: &PreviewServer, method: &str, target: &str, extra: &[(&str, &str)]) -> Response {
    let mut headers: HashMap<String, String> = extra.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    headers.entry("host".into()).or_insert_with(|| format!("127.0.0.1:{}", s.port));
    s.handle(&Request { method: method.into(), target: target.into(), headers })
}

fn path_of(url: &str) -> String {
    url.split_once("://").unwrap().1.split_once('/').map(|(_, rest)| format!("/{rest}")).unwrap()
}

#[test]
fn mime_types_follow_the_extension() {
    assert_eq!(mime_of("/a/index.HTML"), "text/html; charset=utf-8");
    assert_eq!(mime_of("/a/app.mjs"), "text/javascript; charset=utf-8");
    assert_eq!(mime_of("/a/x.png"), "image/png");
    assert_eq!(mime_of("/a/x.svg"), "image/svg+xml");
    assert_eq!(mime_of("/a/README.md"), "text/plain; charset=utf-8");
    assert_eq!(mime_of("/a/x.wasm"), "application/wasm");
    assert_eq!(mime_of("/a/noext"), "application/octet-stream");
    assert_eq!(mime_of("/a/x.exe"), "application/octet-stream");
}

#[test]
fn percent_coding_round_trips_and_rejects_bad_escapes() {
    assert_eq!(encode_component("a b/ç.png"), "a%20b%2F%C3%A7.png");
    assert_eq!(decode_component("a%20b%C3%A7").as_deref(), Some("a bç"));
    assert!(decode_component("%zz").is_none());
    assert!(decode_component("%4").is_none());
    assert!(decode_component("%FF").is_none());
}

#[test]
fn ranges_match_the_electron_server() {
    assert_eq!(parse_range(None, 10), Ok(None));
    assert_eq!(parse_range(Some("bytes=-"), 10), Ok(None));
    assert_eq!(parse_range(Some("items=0-1"), 10), Ok(None));
    assert_eq!(parse_range(Some("bytes=2-4"), 10), Ok(Some((2, 4))));
    assert_eq!(parse_range(Some("bytes=2-"), 10), Ok(Some((2, 9))));
    assert_eq!(parse_range(Some("bytes=-3"), 10), Ok(Some((7, 9))));
    assert_eq!(parse_range(Some("bytes=5-99"), 10), Ok(Some((5, 9))));
    assert_eq!(parse_range(Some("bytes=10-"), 10), Err(()));
    assert_eq!(parse_range(Some("bytes=4-2"), 10), Err(()));
    assert_eq!(parse_range(Some("bytes=0-"), 0), Err(()));
}

#[test]
fn the_request_head_parses_method_target_and_headers() {
    let r = Request::parse("GET /t/a.html?x=1 HTTP/1.1\r\nHost: 127.0.0.1:9\r\nRange: bytes=0-1\r\n\r\n").unwrap();
    assert_eq!((r.method.as_str(), r.target.as_str()), ("GET", "/t/a.html?x=1"));
    assert_eq!(r.header("host"), Some("127.0.0.1:9"));
    assert_eq!(r.header("range"), Some("bytes=0-1"));
    assert!(Request::parse("GET /\r\n\r\n").is_none());
    assert!(Request::parse("GET / SPDY/3\r\n\r\n").is_none());
    assert!(Request::parse("GET / HTTP/1.1\r\nbroken\r\n\r\n").is_none());
}

#[test]
fn a_folder_keeps_one_random_token_and_changes_find_it() {
    let s = PreviewServer::new(9);
    let a = s.register("/x/y").unwrap();
    assert_eq!(s.register("/x/y").unwrap(), a);
    assert_eq!(a.len(), 32);
    assert!(a.bytes().all(|b| b.is_ascii_hexdigit()));
    let b = s.register("/x/z").unwrap();
    assert_ne!(a, b);
    assert_eq!(s.tokens_under("/x/y"), vec![a.clone()]);
    assert_eq!(s.tokens_under("/x/y/deep"), vec![a.clone()]);
    let mut both = s.tokens_under("/x");
    both.sort();
    let mut want = vec![a, b];
    want.sort();
    assert_eq!(both, want);
    assert!(s.tokens_under("/other").is_empty());
}

#[test]
fn urls_point_inside_the_deepest_root() {
    let dir = scratch("url");
    fs::write(join(&dir, "sub/a b.html"), b"<p>").unwrap();
    let s = PreviewServer::new(9);
    let t = s.target_for(&join(&dir, "sub/a b.html"), &[dir.clone()]).unwrap();
    assert_eq!(t.root, dir);
    assert_eq!(t.url, format!("http://127.0.0.1:9/{}/sub/a%20b.html", t.token));
    let own = s.target_for(&join(&dir, "sub/a b.html"), &[]).unwrap();
    assert_eq!(own.root, join(&dir, "sub"));
    assert!(own.url.ends_with("/a%20b.html"));
    let folder = s.target_for(&join(&dir, "sub"), &[dir.clone()]).unwrap();
    assert!(folder.url.ends_with(&format!("{}/sub/", folder.token)));
    let top = s.target_for(&dir, &[dir.clone()]).unwrap();
    assert_eq!(top.url, format!("http://127.0.0.1:9/{}/", top.token));
    assert!(s.target_for("sub/a b.html", &[]).is_none());
    assert!(s.target_for(&join(&dir, "gone.html"), &[]).is_none());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn requests_are_checked_for_method_host_token_and_escape() {
    let dir = scratch("answer");
    fs::write(join(&dir, "sub/a.png"), b"png").unwrap();
    fs::write(join(&dir, "secret.txt"), b"x").unwrap();
    let s = PreviewServer::new(9);
    let tok = s.register(&join(&dir, "sub")).unwrap();
    let ok = get(&s, "GET", &format!("/{tok}/a.png?r=1#x"), &[]);
    assert_eq!(ok.status, 200);
    assert_eq!(ok.body, Body::File { path: join(&dir, "sub/a.png"), start: 0, len: 3 });
    assert_eq!(ok.header("Content-Type"), Some("image/png"));
    assert_eq!(ok.header("Cross-Origin-Resource-Policy"), Some("cross-origin"));
    assert_eq!(ok.header("Cache-Control"), Some("no-store"));
    assert_eq!(ok.header("X-Content-Type-Options"), Some("nosniff"));
    let head = get(&s, "HEAD", &format!("/{tok}/a.png"), &[("host", "localhost:9")]);
    assert_eq!((head.status, &head.body, head.header("Content-Length")), (200, &Body::Empty, Some("3")));
    let post = get(&s, "POST", &format!("/{tok}/a.png"), &[]);
    assert_eq!((post.status, post.header("Allow")), (405, Some("GET, HEAD")));
    assert_eq!(get(&s, "GET", &format!("/{tok}/a.png"), &[("host", "evil.test:9")]).status, 403);
    assert_eq!(get(&s, "GET", "/nope/a.png", &[]).status, 404);
    for bad in ["../secret.txt", "%2E%2E/secret.txt", "..%2Fsecret.txt", "a.png%00", "%5C..%5Csecret.txt"] {
        assert_eq!(get(&s, "GET", &format!("/{tok}/{bad}"), &[]).status, 404, "{bad}");
    }
    assert_eq!(get(&s, "GET", &format!("/{tok}/%zz"), &[]).status, 400);
    assert_eq!(get(&s, "GET", "*", &[]).status, 400);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn folders_redirect_to_a_slash_and_serve_only_their_index() {
    let dir = scratch("dir");
    fs::create_dir_all(join(&dir, "site")).unwrap();
    fs::write(join(&dir, "site/index.html"), b"<h1>").unwrap();
    let s = PreviewServer::new(9);
    let tok = s.register(&dir).unwrap();
    let moved = get(&s, "GET", &format!("/{tok}/site?q=1"), &[]);
    assert_eq!((moved.status, moved.header("Location")), (301, Some(format!("/{tok}/site/?q=1").as_str())));
    let index = get(&s, "GET", &format!("/{tok}/site/"), &[]);
    assert_eq!(index.status, 200);
    assert_eq!(index.header("Content-Type"), Some("text/html; charset=utf-8"));
    assert_eq!(index.header("Cross-Origin-Resource-Policy"), Some("same-origin"));
    assert_eq!(get(&s, "GET", &format!("/{tok}/sub/"), &[]).status, 404);
    assert_eq!(get(&s, "GET", &format!("/{tok}"), &[]).status, 301);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn root_relative_assets_follow_the_referring_page() {
    let s = PreviewServer::new(9);
    let tok = s.register("/x/site").unwrap();
    let from = format!("http://127.0.0.1:9/{tok}/index.html");
    let r = get(&s, "GET", "/app.css?v=2", &[("referer", &from)]);
    assert_eq!((r.status, r.header("Location")), (307, Some(format!("/{tok}/app.css?v=2").as_str())));
    assert_eq!(get(&s, "GET", "/app.css", &[]).status, 404);
    assert_eq!(get(&s, "GET", "/app.css", &[("referer", &format!("http://127.0.0.1:10/{tok}/"))]).status, 404);
    assert_eq!(get(&s, "GET", "/app.css", &[("referer", "http://127.0.0.1:9/unknown/")]).status, 404);
}

#[test]
fn ranges_give_partial_content_or_416() {
    let dir = scratch("range");
    fs::write(join(&dir, "v.mp4"), b"0123456789").unwrap();
    let s = PreviewServer::new(9);
    let tok = s.register(&dir).unwrap();
    let part = get(&s, "GET", &format!("/{tok}/v.mp4"), &[("range", "bytes=2-4")]);
    assert_eq!(part.status, 206);
    assert_eq!(part.header("Content-Range"), Some("bytes 2-4/10"));
    assert_eq!(part.body, Body::File { path: join(&dir, "v.mp4"), start: 2, len: 3 });
    let bad = get(&s, "GET", &format!("/{tok}/v.mp4"), &[("range", "bytes=20-")]);
    assert_eq!((bad.status, bad.header("Content-Range")), (416, Some("bytes */10")));
    let _ = fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn a_link_out_of_the_root_is_not_served() {
    let dir = scratch("link");
    fs::write(join(&dir, "out.png"), b"png").unwrap();
    std::os::unix::fs::symlink(join(&dir, "out.png"), join(&dir, "sub/in.png")).unwrap();
    std::os::unix::fs::symlink(&dir, join(&dir, "sub/up")).unwrap();
    let s = PreviewServer::new(9);
    let tok = s.register(&join(&dir, "sub")).unwrap();
    assert_eq!(get(&s, "GET", &format!("/{tok}/in.png"), &[]).status, 404);
    assert_eq!(get(&s, "GET", &format!("/{tok}/up/out.png"), &[]).status, 404);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_server_answers_over_http_on_loopback() {
    let dir = scratch("http");
    fs::write(join(&dir, "sub/r.txt"), b"HELLO").unwrap();
    let s = start().unwrap();
    let t = s.target_for(&join(&dir, "sub/r.txt"), &[]).unwrap();
    let port = s.port;
    let fetch = |req: String| {
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        c.write_all(req.as_bytes()).unwrap();
        let mut out = String::new();
        c.read_to_string(&mut out).unwrap();
        out
    };
    let ok = fetch(format!("GET {} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n", path_of(&t.url)));
    assert!(ok.starts_with("HTTP/1.1 200 OK\r\n"), "{ok}");
    assert!(ok.contains("Content-Type: text/plain; charset=utf-8\r\n") && ok.contains("Content-Length: 5\r\n"));
    assert!(ok.ends_with("\r\n\r\nHELLO"));
    let part = fetch(format!("GET {} HTTP/1.1\r\nHost: localhost:{port}\r\nRange: bytes=1-2\r\n\r\n", path_of(&t.url)));
    assert!(part.starts_with("HTTP/1.1 206 ") && part.ends_with("\r\n\r\nEL"), "{part}");
    let denied = fetch(format!("GET {} HTTP/1.1\r\nHost: attacker.test\r\n\r\n", path_of(&t.url)));
    assert!(denied.starts_with("HTTP/1.1 403 ") && denied.ends_with("403\n"));
    let _ = fs::remove_dir_all(&dir);
}
