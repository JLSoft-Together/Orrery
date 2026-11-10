use regress::Regex;
use std::sync::OnceLock;

pub struct Js {
    cell: OnceLock<Regex>,
    src: &'static str,
    flags: &'static str,
}

pub type Groups = Vec<Option<String>>;

impl Js {
    pub const fn new(src: &'static str, flags: &'static str) -> Self {
        Js { cell: OnceLock::new(), src, flags }
    }

    pub fn re(&self) -> &Regex {
        self.cell.get_or_init(|| Regex::with_flags(self.src, self.flags).expect("regex"))
    }

    pub fn test(&self, text: &str) -> bool {
        self.re().find(text).is_some()
    }

    pub fn exec(&self, text: &str) -> Option<Groups> {
        self.re().find(text).map(|m| groups(&m, text))
    }

    pub fn all(&self, text: &str) -> Vec<Groups> {
        self.re().find_iter(text).map(|m| groups(&m, text)).collect()
    }

    pub fn replace_all(&self, text: &str, rep: &str) -> String {
        self.re().replace_all(text, rep)
    }
}

pub fn groups(m: &regress::Match, text: &str) -> Groups {
    m.groups().map(|g| g.map(|r| text[r].to_string())).collect()
}

pub fn dynamic(src: &str) -> Option<Regex> {
    Regex::new(src).ok()
}

pub fn g(v: &Groups, i: usize) -> Option<&str> {
    v.get(i).and_then(|x| x.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_and_backrefs() {
        static R: Js = Js::new(r"<(a|b)>(x)?<\/\1>", "");
        let m = R.exec("<b></b>").unwrap();
        assert_eq!(m, vec![Some("<b></b>".into()), Some("b".into()), None]);
        assert!(!R.test("<a></b>"));
        static S: Js = Js::new(r"(^|[^:])\/\/.*$", "gm");
        assert_eq!(S.replace_all("a // x\nhttp://y", "$1"), "a \nhttp://y");
    }
}
