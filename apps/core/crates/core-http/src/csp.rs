use std::path::Path;

use sha2::{Digest, Sha256};

use crate::js;

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16) | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8) | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn tag_has_src(lower: &str, from: usize, to: usize) -> bool {
    let bytes = lower.as_bytes();
    let tag = &lower[from..to];
    let mut at = 0;
    while let Some(i) = tag[at..].find("src=") {
        let j = at + i;
        if is_word(bytes[from + j - 1]) {
            at = j + 1;
            continue;
        }
        return true;
    }
    false
}

fn inline_script_bodies(html: &str) -> Vec<&str> {
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(off) = lower[i..].find("<script") {
        let start = i + off;
        let attrs = start + 7;
        let Some(gt) = lower[attrs..].find('>').map(|g| attrs + g) else { break };
        if tag_has_src(&lower, attrs, gt) {
            i = start + 1;
            continue;
        }
        let body = gt + 1;
        match lower[body..].find("</script>") {
            Some(end) => {
                out.push(&html[body..body + end]);
                i = body + end + "</script>".len();
            }
            None => i = start + 1,
        }
    }
    out
}

fn normalize_newlines(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    out
}

pub fn content_security_policy(html: &str) -> String {
    let hashes: Vec<String> = inline_script_bodies(html)
        .into_iter()
        .filter(|body| !js::trim(body).is_empty())
        .map(normalize_newlines)
        .map(|body| format!("'sha256-{}'", base64(&Sha256::digest(body.as_bytes()))))
        .collect();
    let script_src = format!("script-src 'self' {}", hashes.join(" "));
    [
        "default-src 'self'",
        script_src.trim(),
        "style-src 'self' 'unsafe-inline'",
        "img-src 'self' data: blob: http://127.0.0.1:* http://localhost:*",
        "font-src 'self' data:",
        "connect-src 'self' ws://127.0.0.1:* ipc: http://ipc.localhost",
        "worker-src 'self'",
        "frame-src http://127.0.0.1:* http://localhost:* https: http:",
        "object-src 'none'",
        "base-uri 'none'",
        "form-action 'none'",
        "frame-ancestors 'none'",
    ]
    .join("; ")
}

fn hashed_asset(name: &str) -> bool {
    let Some((stem, ext)) = name.rsplit_once('.') else { return false };
    if ext.is_empty() || !ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return false;
    }
    let b = stem.as_bytes();
    b.len() >= 9 && b[b.len() - 9] == b'-' && b[b.len() - 8..].iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-')
}

pub fn cache_control_for(root: &Path, file: &Path) -> &'static str {
    let in_assets = file.parent() == Some(root.join("assets").as_path());
    let hashed = file.file_name().and_then(|n| n.to_str()).is_some_and(hashed_asset);
    if in_assets && hashed {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(s: &str) -> String {
        base64(&Sha256::digest(s.as_bytes()))
    }

    fn at(parts: &[&str]) -> &'static str {
        let root = Path::new("dist-root");
        cache_control_for(root, &parts.iter().fold(root.to_path_buf(), |p, s| p.join(s)))
    }

    #[test]
    fn base64_matches_the_standard_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(&Sha256::digest(b"abc")), "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=");
    }

    #[test]
    fn cache_control_makes_only_vite_hashed_bundles_immutable() {
        let immutable = "public, max-age=31536000, immutable";
        assert_eq!(at(&["assets", "index-BMrYeALK.js"]), immutable);
        assert_eq!(at(&["assets", "WorkspaceView-C61tNg0-.css"]), immutable);
        assert_eq!(at(&["assets", "be-vietnam-pro-latin-400-normal-PpnXBOrz.woff2"]), immutable);
    }

    #[test]
    fn cache_control_revalidates_public_files_also_those_copied_under_assets() {
        assert_eq!(at(&["assets", "art", "painting-abstract.png"]), "no-cache");
        assert_eq!(at(&["assets", "brand", "logo-12345678.png"]), "no-cache");
        assert_eq!(at(&["assets", "manifest.json"]), "no-cache");
        assert_eq!(at(&["assets", "og-image.png"]), "no-cache");
        assert_eq!(at(&["sw.js"]), "no-cache");
        assert_eq!(at(&["icon-192.png"]), "no-cache");
    }

    #[test]
    fn csp_allows_only_the_bundle_and_the_hashed_inline_script() {
        let inline = "document.documentElement.classList.add('dark')";
        let csp = content_security_policy(&format!("<head><script>{inline}</script><script type=\"module\" src=\"/assets/i.js\"></script></head>"));
        assert!(csp.contains(&format!("script-src 'self' 'sha256-{}'", hash(inline))), "{csp}");
        assert!(!csp.contains("unsafe-eval"));
        assert!(csp.contains("connect-src 'self' ws://127.0.0.1:* ipc: http://ipc.localhost"));
        assert!(csp.contains("object-src 'none'"));
    }

    #[test]
    fn csp_lets_workspace_browser_panes_frame_web_pages_and_show_loopback_images_nothing_more() {
        let csp = content_security_policy("<script src=\"/a.js\"></script>");
        assert!(csp.contains("frame-src http://127.0.0.1:* http://localhost:* https: http:"));
        assert!(csp.contains("img-src 'self' data: blob: http://127.0.0.1:* http://localhost:*"));
        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("frame-ancestors 'none'"));
        for directive in csp.split("; ").filter(|d| d.starts_with("script-src") || d.starts_with("connect-src")) {
            assert!(!directive.contains("https:"), "{directive}");
        }
    }

    #[test]
    fn csp_hashes_a_crlf_inline_script_the_way_the_browser_parses_it() {
        let csp = content_security_policy("<script>\r\n  a()\r\n</script>");
        assert!(csp.contains(&format!("'sha256-{}'", hash("\n  a()\n"))), "{csp}");
        let lone_cr = content_security_policy("<script>\r  a()\r</script>");
        assert!(lone_cr.contains(&format!("'sha256-{}'", hash("\n  a()\n"))));
    }

    #[test]
    fn csp_has_no_hash_without_inline_scripts() {
        assert!(content_security_policy("<script src=\"/a.js\"></script>").contains("script-src 'self';"));
        assert!(content_security_policy("<script>  \n </script>").contains("script-src 'self';"));
    }

    #[test]
    fn csp_scanner_follows_the_regex_rules() {
        let html = "<SCRIPT type=module>x=1</SCRIPT><script data-nosrc=1>y=2</script><script data-src=1>z</script><script crossorigin src=/x.js></script>";
        let csp = content_security_policy(html);
        assert!(csp.contains(&hash("x=1")));
        assert!(csp.contains(&hash("y=2")));
        assert!(!csp.contains(&hash("z")));
    }
}
