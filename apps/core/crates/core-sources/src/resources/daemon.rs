use core_transcript::jsstr::js_space;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DaemonInfo {
    pub kind: &'static str,
    pub version: Option<String>,
    pub jdk: Option<String>,
    pub avd: Option<String>,
    pub serial: Option<String>,
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub fn base_name(name: &str) -> String {
    let lower = name.to_lowercase();
    lower.strip_suffix(".exe").map(str::to_owned).unwrap_or(lower)
}

const DAEMON_HOSTS: [&str; 6] = ["java", "javaw", "dart", "dartaotruntime", "dartvm", "emulator"];

pub fn may_be_daemon(name: &str) -> bool {
    let n = base_name(name);
    DAEMON_HOSTS.contains(&n.as_str()) || n.starts_with("qemu-system")
}

fn at(c: &[char], i: usize, w: &str, ci: bool) -> bool {
    let n = w.chars().count();
    i + n <= c.len() && c[i..i + n].iter().zip(w.chars()).all(|(a, b)| if ci { a.is_ascii() && a.eq_ignore_ascii_case(&b) } else { *a == b })
}

fn digits(c: &[char], i: usize) -> usize {
    let mut j = i;
    while j < c.len() && c[j].is_ascii_digit() {
        j += 1;
    }
    j
}

fn word_dot_run(c: &[char], i: usize) -> usize {
    let mut j = i;
    while j < c.len() && (is_word(c[j]) || c[j] == '.') {
        j += 1;
    }
    j
}

fn dotted_ends(c: &[char], i: usize) -> Vec<usize> {
    let d = digits(c, i);
    if d == i {
        return Vec::new();
    }
    let mut ends = Vec::new();
    let mut j = d;
    while c.get(j) == Some(&'.') {
        let e = digits(c, j + 1);
        if e == j + 1 {
            break;
        }
        j = e;
        ends.push(j);
    }
    ends.reverse();
    ends
}

fn version_candidates(c: &[char], i: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for end in dotted_ends(c, i) {
        if c.get(end) == Some(&'-') {
            let run_end = word_dot_run(c, end + 1);
            for e in (end + 2..=run_end).rev() {
                out.push(e);
            }
        }
        out.push(end);
    }
    out
}

fn gradle_version(c: &[char]) -> Option<String> {
    let mut i = 0;
    while i < c.len() {
        if at(c, i, "GradleDaemon", false) {
            let mut j = i + 12;
            let ws = j;
            while j < c.len() && js_space(c[j]) {
                j += 1;
            }
            if j > ws {
                if let Some(&end) = version_candidates(c, j).first() {
                    return Some(c[j..end].iter().collect());
                }
            }
        }
        i += 1;
    }
    None
}

fn gradle_cp_version(c: &[char]) -> Option<String> {
    for i in 0..c.len() {
        if !at(c, i, "gradle-", false) {
            continue;
        }
        let after = i + 7;
        for lead in ["launcher-", "daemon-main-"] {
            if at(c, after, lead, false) {
                let v = after + lead.len();
                for end in version_candidates(c, v) {
                    if at(c, end, ".jar", false) {
                        return Some(c[v..end].iter().collect());
                    }
                }
            }
        }
        for end in version_candidates(c, after) {
            if c.get(end).is_some_and(|x| *x == '\\' || *x == '/') && at(c, end + 1, "lib", false) && c.get(end + 4).is_some_and(|x| *x == '\\' || *x == '/') {
                return Some(c[after..end].iter().collect());
            }
        }
    }
    None
}

fn kotlin_candidates(c: &[char], i: usize) -> Vec<usize> {
    let d = digits(c, i);
    if d == i || c.get(d) != Some(&'.') {
        return Vec::new();
    }
    let m = digits(c, d + 1);
    if m == d + 1 {
        return Vec::new();
    }
    let mut bases = Vec::new();
    if c.get(m) == Some(&'.') {
        let p = digits(c, m + 1);
        if p > m + 1 {
            bases.push(p);
        }
    }
    bases.push(m);
    let mut out = Vec::new();
    for base in bases {
        if c.get(base) == Some(&'-') {
            let mut e = base + 1;
            while e < c.len() && is_word(c[e]) {
                e += 1;
            }
            for x in (base + 2..=e).rev() {
                out.push(x);
            }
        }
        out.push(base);
    }
    out
}

fn kotlin_version(c: &[char]) -> Option<String> {
    for i in 0..c.len() {
        if !at(c, i, "kotlin-", false) {
            continue;
        }
        let after = i + 7;
        for lead in ["compiler-embeddable-", "compiler-", "daemon-", "daemon-embeddable-"] {
            if at(c, after, lead, false) {
                let v = after + lead.len();
                for end in kotlin_candidates(c, v) {
                    if at(c, end, ".jar", false) {
                        return Some(c[v..end].iter().collect());
                    }
                }
            }
        }
    }
    None
}

fn jdk_of(c: &[char]) -> Option<String> {
    let delim = |x: char| x == '\\' || x == '/' || x == '"' || js_space(x);
    let sep = |x: char| x == '\\' || x == '/';
    let mut i = 0;
    while i < c.len() {
        if delim(c[i]) {
            i += 1;
            continue;
        }
        let mut e = i;
        while e < c.len() && !delim(c[e]) {
            e += 1;
        }
        if c.get(e).is_some_and(|x| sep(*x)) && jdk_tail(c, e + 1) {
            return Some(c[i..e].iter().collect());
        }
        i = e;
    }
    None
}

fn jdk_tail(c: &[char], start: usize) -> bool {
    let sep = |x: char| x == '\\' || x == '/';
    let mut starts = Vec::new();
    for with_home in [true, false] {
        let mut p = start;
        if with_home {
            if at(c, p, "Contents", true) && c.get(p + 8).is_some_and(|x| sep(*x)) && at(c, p + 9, "Home", true) && c.get(p + 13).is_some_and(|x| sep(*x)) {
                p += 14;
            } else {
                continue;
            }
        }
        for with_jre in [true, false] {
            let mut q = p;
            if with_jre {
                if at(c, q, "jre", true) && c.get(q + 3).is_some_and(|x| sep(*x)) {
                    q += 4;
                } else {
                    continue;
                }
            }
            starts.push(q);
        }
    }
    for q in starts {
        if !(at(c, q, "bin", true) && c.get(q + 3).is_some_and(|x| sep(*x)) && at(c, q + 4, "java", true)) {
            continue;
        }
        let base = q + 8;
        for w in [true, false] {
            let mut e = base;
            if w {
                if c.get(e).is_some_and(|x| x.eq_ignore_ascii_case(&'w')) {
                    e += 1;
                } else {
                    continue;
                }
            }
            for exe in [true, false] {
                let mut f = e;
                if exe {
                    if at(c, f, ".exe", true) {
                        f += 4;
                    } else {
                        continue;
                    }
                }
                if f >= c.len() || c[f] == '"' || js_space(c[f]) {
                    return true;
                }
            }
        }
    }
    false
}

fn flag_start(c: &[char], q: usize, flag: &str) -> Option<usize> {
    if !at(c, q, flag, false) || !(q == 0 || js_space(c[q - 1])) {
        return None;
    }
    let mut j = q + flag.chars().count();
    let ws = j;
    while j < c.len() && js_space(c[j]) {
        j += 1;
    }
    (j > ws).then_some(j)
}

fn avd_value(c: &[char]) -> Option<String> {
    for q in 0..c.len() {
        let Some(mut j) = flag_start(c, q, "-avd") else { continue };
        if c.get(j) == Some(&'"') {
            j += 1;
        }
        let s = j;
        while j < c.len() && j - s < 100 && avd_char(c[j]) {
            j += 1;
        }
        if j > s {
            return Some(c[s..j].iter().collect());
        }
    }
    None
}

fn port_value(c: &[char]) -> Option<String> {
    for q in 0..c.len() {
        let Some(j) = flag_start(c, q, "-port") else { continue };
        let e = digits(c, j);
        if (4..=5).contains(&(e - j)) && c.get(e).is_none_or(|x| js_space(*x)) {
            return Some(c[j..e].iter().collect());
        }
    }
    None
}
fn avd_char(x: char) -> bool {
    x.is_ascii_alphanumeric() || matches!(x, '.' | '_' | '-')
}

fn has_word_bounded(c: &[char], w: &str) -> bool {
    (0..c.len()).any(|i| at(c, i, w, false) && (i == 0 || !is_word(c[i - 1])) && c.get(i + w.chars().count()).is_none_or(|x| !is_word(*x)))
}

fn is_flutter_daemon(c: &[char]) -> bool {
    let snap = "flutter_tools.snapshot";
    let n = snap.chars().count();
    let a = (0..c.len()).any(|i| at(c, i, snap, false) && c.get(i + n).is_none_or(|x| !is_word(*x)));
    a && (0..c.len()).any(|i| js_space(c[i]) && at(c, i + 1, "daemon", false) && c.get(i + 7).is_none_or(|x| js_space(*x)))
}

fn is_dart_server(c: &[char]) -> bool {
    has_word_bounded(c, "analysis_server") || (0..c.len()).any(|i| js_space(c[i]) && at(c, i + 1, "language-server", false) && c.get(i + 16).is_none_or(|x| js_space(*x)))
}

fn contains(c: &[char], w: &str) -> bool {
    (0..c.len()).any(|i| at(c, i, w, false))
}

pub fn daemon_of(name: &str, cmd: Option<&str>) -> Option<DaemonInfo> {
    let cmd = cmd.filter(|c| !c.is_empty())?;
    if !may_be_daemon(name) {
        return None;
    }
    let n = base_name(name);
    let c: Vec<char> = cmd.chars().collect();
    if n == "emulator" || n.starts_with("qemu-system") {
        let avd = avd_value(&c);
        let port = port_value(&c);
        if avd.is_none() && port.is_none() {
            return None;
        }
        return Some(DaemonInfo { kind: "emulator", avd, serial: port.map(|p| format!("emulator-{p}")), ..Default::default() });
    }
    if contains(&c, "org.gradle.launcher.daemon.bootstrap.GradleDaemon") {
        let version = gradle_version(&c).or_else(|| gradle_cp_version(&c));
        return Some(DaemonInfo { kind: "gradle", version, jdk: jdk_of(&c), ..Default::default() });
    }
    if contains(&c, "KotlinCompileDaemon") {
        return Some(DaemonInfo { kind: "kotlin", version: kotlin_version(&c), jdk: jdk_of(&c), ..Default::default() });
    }
    if is_flutter_daemon(&c) {
        return Some(DaemonInfo { kind: "flutter", ..Default::default() });
    }
    if is_dart_server(&c) {
        return Some(DaemonInfo { kind: "dart", ..Default::default() });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(name: &str, cmd: &str) -> Option<DaemonInfo> {
        daemon_of(name, Some(cmd))
    }

    #[test]
    fn names_gradle_kotlin_dart_flutter_and_the_emulator() {
        let gradle = "\"C:\\Program Files\\Java\\jdk-17.0.12\\bin\\java.exe\" -cp C:\\Users\\me\\.gradle\\wrapper\\dists\\gradle-8.9-bin\\x\\gradle-8.9\\lib\\gradle-daemon-main-8.9.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.9";
        let g = d("java.exe", gradle).unwrap();
        assert_eq!((g.kind, g.version.as_deref(), g.jdk.as_deref()), ("gradle", Some("8.9"), Some("jdk-17.0.12")));
        let k = d("java.exe", "java -cp kotlin-compiler-embeddable-2.0.21.jar org.jetbrains.kotlin.daemon.KotlinCompileDaemon").unwrap();
        assert_eq!((k.kind, k.version.as_deref(), k.jdk), ("kotlin", Some("2.0.21"), None));
        assert_eq!(d("dart.exe", "dart.exe language-server --protocol=lsp").unwrap().kind, "dart");
        assert_eq!(d("dart.exe", "dart flutter_tools.snapshot daemon").unwrap().kind, "flutter");
        assert!(d("dart.exe", "dart flutter_tools.snapshot run").is_none());
        let e = d("qemu-system-x86_64.exe", "qemu-system-x86_64.exe -avd Pixel_7_API_34 -port 5556").unwrap();
        assert_eq!((e.kind, e.avd.as_deref(), e.serial.as_deref()), ("emulator", Some("Pixel_7_API_34"), Some("emulator-5556")));
        assert!(d("java.exe", "java -jar app.jar").is_none());
        assert!(d("node.exe", "node GradleDaemon").is_none());
        assert!(daemon_of("java.exe", None).is_none());
    }

    #[test]
    fn gradle_version_falls_back_to_the_classpath_and_the_jdk_folder_form() {
        let mac = "/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin/java -cp x org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.14";
        let g = d("java", mac).unwrap();
        assert_eq!((g.version.as_deref(), g.jdk.as_deref()), (Some("8.14"), Some("jbr")));
        let cp = d("java", "/opt/jdk/bin/java -cp /x/gradle-launcher-8.5.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon").unwrap();
        assert_eq!((cp.version.as_deref(), cp.jdk.as_deref()), (Some("8.5"), Some("jdk")));
        let lib = d("java", "java -cp /x/gradle-8.7-rc-1/lib/a.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon").unwrap();
        assert_eq!(lib.version, None);
        let lib2 = d("java", "java -cp /x/gradle-8.7-rc/lib/a.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon").unwrap();
        assert_eq!(lib2.version.as_deref(), Some("8.7-rc"));
    }

    #[test]
    fn emulator_flags_follow_the_regex_rules() {
        assert_eq!(d("emulator", "emulator -avd \"Pixel\" -port 5554").unwrap().avd.as_deref(), Some("Pixel"));
        assert_eq!(d("emulator", "emulator -port 55555").unwrap().serial.as_deref(), Some("emulator-55555"));
        assert!(d("emulator", "emulator -port 555555").is_none());
        assert!(d("emulator", "emulator -port 555").is_none());
        assert!(d("emulator", "emulator x-avd Pixel").is_none());
        assert_eq!(d("emulator", "-avd A").unwrap().avd.as_deref(), Some("A"));
    }
}
