use crate::http::Response;
use crate::origin::has_app_key;
use crate::query::Params;

pub const COOKIE: &str = "ccm_app";
const KEY_MAX: usize = 256;

pub fn cookie_value<'a>(headers: impl Iterator<Item = &'a str>, name: &str) -> Option<&'a str> {
    headers.flat_map(|h| h.split(';')).filter_map(|pair| pair.trim().split_once('=')).find(|(k, _)| k.trim() == name).map(|(_, v)| v.trim())
}

pub fn authorized<'a>(header: Option<&str>, cookies: impl Iterator<Item = &'a str>, key: Option<&str>) -> bool {
    if has_app_key(header, key) {
        return true;
    }
    key.is_some() && cookie_value(cookies, COOKIE).is_some_and(|got| has_app_key(Some(got), key))
}

fn cookie_safe(key: &str) -> bool {
    !key.is_empty() && key.len() <= KEY_MAX && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn location(query: &str) -> String {
    let rest: Vec<&str> = query.split('&').filter(|p| !p.is_empty() && *p != "k" && !p.starts_with("k=")).collect();
    if rest.is_empty() {
        "/".to_owned()
    } else {
        format!("/?{}", rest.join("&"))
    }
}

pub fn login(query: &str, key: Option<&str>) -> Option<Response> {
    let params = Params::parse(query);
    let given = params.get("k")?;
    let redirect = |r: Response| r.header("location", &location(query)).header("cache-control", "no-store").header("referrer-policy", "no-referrer");
    let Some(key) = key else { return Some(redirect(Response::empty(302))) };
    if !cookie_safe(key) || !has_app_key(Some(given), Some(key)) {
        return Some(Response::text(403, "text/plain; charset=utf-8", "Orrery only opens in its desktop app.").header("cache-control", "no-store"));
    }
    Some(redirect(Response::empty(302)).header("set-cookie", &format!("{COOKIE}={key}; HttpOnly; SameSite=Strict; Path=/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header<'a>(r: &'a Response, name: &str) -> Option<&'a str> {
        r.headers.iter().find(|(k, _)| *k == name).map(|(_, v)| v.as_str())
    }

    #[test]
    fn finds_the_cookie_among_others_and_across_headers() {
        assert_eq!(cookie_value(["a=1; ccm_app=xyz ; b=2"].into_iter(), COOKIE), Some("xyz"));
        assert_eq!(cookie_value(["a=1", "ccm_app=k2"].into_iter(), COOKIE), Some("k2"));
        assert_eq!(cookie_value(["xccm_app=no; ccm_apps=no"].into_iter(), COOKIE), None);
        assert_eq!(cookie_value(std::iter::empty(), COOKIE), None);
    }

    #[test]
    fn the_cookie_or_the_header_authorizes() {
        let key = Some("s3cret");
        assert!(authorized(Some("s3cret"), std::iter::empty(), key));
        assert!(authorized(None, ["ccm_app=s3cret"].into_iter(), key));
        assert!(authorized(Some("wrong"), ["ccm_app=s3cret"].into_iter(), key));
        assert!(!authorized(None, ["ccm_app=s3cre"].into_iter(), key));
        assert!(!authorized(None, ["ccm_app=s3crets"].into_iter(), key));
        assert!(!authorized(None, ["other=s3cret"].into_iter(), key));
        assert!(!authorized(None, std::iter::empty(), key));
        assert!(authorized(None, std::iter::empty(), None));
    }

    #[test]
    fn a_right_key_sets_the_cookie_and_drops_it_from_the_url() {
        let r = login("k=abc123", Some("abc123")).unwrap();
        assert_eq!(r.status, 302);
        assert_eq!(header(&r, "location"), Some("/"));
        assert_eq!(header(&r, "set-cookie"), Some("ccm_app=abc123; HttpOnly; SameSite=Strict; Path=/"));
        assert_eq!(header(&r, "cache-control"), Some("no-store"));
        let compact = login("mode=compact&k=abc123", Some("abc123")).unwrap();
        assert_eq!(header(&compact, "location"), Some("/?mode=compact"));
    }

    #[test]
    fn a_wrong_or_unsafe_key_gets_no_cookie() {
        for (q, key) in [("k=nope", "abc123"), ("k=", "abc123"), ("k=abc12", "abc123"), ("k=a%3Bb", "a;b")] {
            let r = login(q, Some(key)).unwrap();
            assert_eq!(r.status, 403, "{q}");
            assert!(header(&r, "set-cookie").is_none(), "{q}");
        }
        assert!(login("mode=compact", Some("abc123")).is_none());
        assert!(login("", Some("abc123")).is_none());
    }

    #[test]
    fn without_a_key_the_query_only_redirects() {
        let r = login("k=anything", None).unwrap();
        assert_eq!(r.status, 302);
        assert!(header(&r, "set-cookie").is_none());
    }
}
