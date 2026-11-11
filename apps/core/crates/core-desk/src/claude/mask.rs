use regex::Regex;
use std::sync::OnceLock;

const SECRET_NAME: &str = "[A-Za-z0-9_]*(?:TOKEN|SECRET|PASSWORD|PASSWD|PWD|API_?KEY|ACCESS_?KEY|PRIVATE_?KEY|AUTH|CREDENTIALS?|COOKIE|SESSION)[A-Za-z0-9_]*";
const SP: &str = r"[\s\x{feff}]";
const NSP: &str = r"\s\x{feff}";

struct Rules {
    userinfo: Regex,
    scheme: Regex,
    named: Regex,
    flag: Regex,
    query: Regex,
}

fn rules() -> &'static Rules {
    static R: OnceLock<Rules> = OnceLock::new();
    R.get_or_init(|| Rules {
        userinfo: Regex::new(&format!(r"(://)[^/{NSP}:@]+:[^/{NSP}@]+@")).unwrap(),
        scheme: Regex::new(&format!(r"(?i)(?-u:\b)(Bearer|Basic|token){SP}+[A-Za-z0-9._~+/=-]{{8,}}")).unwrap(),
        named: Regex::new(&format!(r#"(?i)(?-u:\b)({SECRET_NAME})({SP}*[=:]{SP}*)("[^"]*"|'[^']*'|[^{NSP};&|"']+)"#)).unwrap(),
        flag: Regex::new(&format!(r#"(?i)(--?[A-Za-z0-9-]*(?:token|secret|password|passwd|api-?key|auth|key)[A-Za-z0-9-]*)(=|{SP}+)("[^"]*"|'[^']*'|[^{NSP};&|"'-][^{NSP};&|"']*)"#)).unwrap(),
        query: Regex::new(r#"(?i)([?&](?:token|key|secret|sig|signature|access_token|api_key|apikey|password)=)[^&\s\x{feff}"']+"#).unwrap(),
    })
}

fn is_count(name: &str, value: &str) -> bool {
    (name.eq_ignore_ascii_case("session") || name.eq_ignore_ascii_case("sessions")) && (1..=4).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit())
}

pub fn mask_hook_text(text: &str, home: &str, platform: &str) -> String {
    let r = rules();
    let out = r.userinfo.replace_all(text, "${1}••••@");
    let out = r.scheme.replace_all(&out, "${1} ••••");
    let out = r.named.replace_all(&out, |c: &regex::Captures| {
        if is_count(&c[1], &c[3]) {
            c[0].to_owned()
        } else {
            format!("{}{}••••", &c[1], &c[2])
        }
    });
    let out = r.flag.replace_all(&out, "${1}${2}••••");
    let out = r.query.replace_all(&out, "${1}••••");
    core_sources::mask::mask_secrets(&out, home, platform).replace("[masked]", "••••")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_credentials() {
        assert_eq!(mask_hook_text("curl https://u:p4ss@h.io/x?token=abc&y=1", "", "darwin"), "curl https://••••@h.io/x?token=••••");
        assert_eq!(mask_hook_text("API_KEY=\"x y\" run --auth-token abc -k", "", "darwin"), "API_KEY=•••• run --auth-token •••• -k");
        assert_eq!(mask_hook_text("x Bearer abcdefghij", "", "darwin"), "x Bearer ••••");
        assert_eq!(mask_hook_text("Session: 1 live\nSESSION=abc session_id: 42 session=48213907", "", "darwin"), "Session: 1 live\nSESSION=•••• session_id: •••• session=••••");
        assert_eq!(mask_hook_text("C:\\Users\\me\\x.ps1", "C:\\Users\\me", "win32"), "~\\x.ps1");
    }
}
