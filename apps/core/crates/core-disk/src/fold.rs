use std::collections::HashMap;
use std::sync::OnceLock;

const GROUPS: [(&str, char); 14] = [
    ("àáâãäåāăąǎǟǡǻȁȃȧạảấầẩẫậắằẳẵặ", 'a'),
    ("çćĉċč", 'c'),
    ("ďđ", 'd'),
    ("èéêëēĕėęěȅȇȩẹẻẽếềểễệ", 'e'),
    ("ìíîïĩīĭįıǐȉȋịỉ", 'i'),
    ("ñńņňŉ", 'n'),
    ("òóôõöøōŏőơǒǫǭǿȍȏȫȭȯȱọỏốồổỗộớờởỡợ", 'o'),
    ("ùúûüũūŭůűųưǔǖǘǚǜȕȗụủứừửữự", 'u'),
    ("ýÿŷỳỵỷỹ", 'y'),
    ("ĝğġģǧ", 'g'),
    ("śŝşšș", 's'),
    ("ţťŧț", 't'),
    ("źżž", 'z'),
    ("ĺļľŀł", 'l'),
];

fn table() -> &'static HashMap<char, char> {
    static T: OnceLock<HashMap<char, char>> = OnceLock::new();
    T.get_or_init(|| {
        let mut m = HashMap::new();
        for (chars, base) in GROUPS {
            for c in chars.chars() {
                m.insert(c, base);
            }
        }
        m.insert('ß', 's');
        m.insert('æ', 'a');
        m.insert('œ', 'o');
        m.insert('þ', 't');
        m
    })
}

fn combining(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
}

pub fn fold_into(s: &str, out: &mut String) {
    out.clear();
    if s.is_ascii() {
        out.extend(s.bytes().map(|b| b.to_ascii_lowercase() as char));
        return;
    }
    let t = table();
    for c in s.chars() {
        if combining(c) {
            continue;
        }
        for l in c.to_lowercase() {
            out.push(*t.get(&l).unwrap_or(&l));
        }
    }
}

pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    fold_into(s, &mut out);
    out
}

pub fn contains_ci(hay: &[u8], needle: &[u8]) -> bool {
    let n = needle.len();
    if n == 0 {
        return true;
    }
    if hay.len() < n {
        return false;
    }
    let first = needle[0];
    let upper = first.to_ascii_uppercase();
    let last = hay.len() - n;
    let mut i = 0;
    while i <= last {
        let b = hay[i];
        if (b == first || b == upper) && hay[i + 1..i + n].eq_ignore_ascii_case(&needle[1..]) {
            return true;
        }
        i += 1;
    }
    false
}

pub fn cmp_ci(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    a.iter().map(u8::to_ascii_lowercase).cmp(b.iter().map(u8::to_ascii_lowercase))
}

fn char_len(b: u8) -> usize {
    match b {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

pub fn glob_ci(text: &[u8], pat: &[u8]) -> bool {
    let (mut t, mut p) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pat.len() && pat[p] == b'*' {
            star = Some((p, t));
            p += 1;
            continue;
        }
        if p < pat.len() && pat[p] == b'?' {
            p += 1;
            t += char_len(text[t]);
            continue;
        }
        if p < pat.len() && pat[p].eq_ignore_ascii_case(&text[t]) {
            p += 1;
            t += 1;
            continue;
        }
        match star {
            Some((sp, st)) => {
                let next = st + char_len(text[st]);
                star = Some((sp, next));
                p = sp + 1;
                t = next;
            }
            None => return false,
        }
    }
    while p < pat.len() && pat[p] == b'*' {
        p += 1;
    }
    p == pat.len()
}

pub const GROUP_NAMES: [&str; 8] = ["other", "image", "video", "audio", "doc", "archive", "installer", "code"];

const EXTS: [(&str, u8); 7] = [
    ("jpg jpeg png gif webp bmp tif tiff heic heif svg ico raw cr2 cr3 nef arw dng psd avif jxl", 1),
    ("mp4 mkv mov avi wmv flv webm m4v mpg mpeg 3gp ts mts m2ts vob", 2),
    ("mp3 wav flac aac ogg m4a wma opus aiff aif mid midi amr", 3),
    ("pdf doc docx xls xlsx ppt pptx txt md rtf odt ods odp csv epub pages numbers key tex log", 4),
    ("zip rar 7z tar gz tgz bz2 xz zst lz4 iso dmg cab jar war", 5),
    ("exe msi msix msixbundle appx appxbundle apk aab xapk ipa pkg deb rpm", 6),
    ("rs ts js mjs cjs tsx jsx vue py java kt kts dart swift go c h cc cpp hpp cs rb php json yaml yml toml xml html htm css scss less sh ps1 bat cmd gradle sql lua m mm gd cmake", 7),
];

fn ext_table() -> &'static HashMap<&'static str, u8> {
    static T: OnceLock<HashMap<&'static str, u8>> = OnceLock::new();
    T.get_or_init(|| EXTS.iter().flat_map(|(list, g)| list.split(' ').map(move |e| (e, *g))).collect())
}

pub fn group_of(name: &str) -> u8 {
    let Some(dot) = name.rfind('.') else { return 0 };
    let ext = &name[dot + 1..];
    if ext.is_empty() || ext.len() > 12 {
        return 0;
    }
    let mut buf = [0u8; 12];
    for (i, b) in ext.bytes().enumerate() {
        buf[i] = b.to_ascii_lowercase();
    }
    let Ok(lower) = std::str::from_utf8(&buf[..ext.len()]) else { return 0 };
    ext_table().get(lower).copied().unwrap_or(0)
}

pub fn group_id(name: &str) -> Option<u8> {
    GROUP_NAMES.iter().position(|g| *g == name).map(|i| i as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_vietnamese_and_nfd() {
        assert_eq!(fold("Nguyễn Văn Đức.JPG"), "nguyen van duc.jpg");
        assert_eq!(fold("Ảnh Đẹp"), "anh dep");
        assert_eq!(fold("Tie\u{0302}\u{0301}ng Vie\u{0323}\u{0302}t"), "tieng viet");
        assert_eq!(fold("ỪỨỬỮỰ"), "uuuuu");
    }

    #[test]
    fn matches_substrings_and_globs() {
        assert!(contains_ci(b"Holiday.MP4", b"day.mp"));
        assert!(!contains_ci(b"abc", b"abd"));
        assert!(glob_ci(b"Clip.MP4", b"*.mp4"));
        assert!(glob_ci(b"a1b.txt", b"a?b.*"));
        assert!(!glob_ci(b"clip.mp40", b"*.mp4"));
        assert!(glob_ci("ảnh.jpg".as_bytes(), b"?nh.jpg"));
    }

    #[test]
    fn groups_by_extension() {
        assert_eq!(group_of("x.JPG"), 1);
        assert_eq!(group_of("movie.mkv"), 2);
        assert_eq!(group_of("setup.exe"), 6);
        assert_eq!(group_of("noext"), 0);
        assert_eq!(group_id("archive"), Some(5));
    }
}
