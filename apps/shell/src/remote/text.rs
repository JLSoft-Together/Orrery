use super::ctx::PairAsk;
use super::devices::valid_platform;
use super::pair::is_hidden;

pub struct RemoteText {
    pub pair_detail: &'static str,
    pub allow: &'static str,
    pub cancel: &'static str,
    pub unknown_title: &'static str,
    pub unknown_body: &'static str,
    pub connected_title: &'static str,
    vi: bool,
}

const EN: RemoteText = RemoteText {
    pair_detail: "This device will see your sessions and control Orrery on this computer. You can revoke it any time in Settings → Remote.",
    allow: "Allow",
    cancel: "Cancel",
    unknown_title: "Orrery Remote",
    unknown_body: "An unknown device tried to connect and was refused.",
    connected_title: "Orrery Remote",
    vi: false,
};

const VI: RemoteText = RemoteText {
    pair_detail: "Thiết bị này sẽ xem các phiên và điều khiển Orrery trên máy này. Thu hồi bất cứ lúc nào trong Cài đặt → Remote.",
    allow: "Cho phép",
    cancel: "Huỷ",
    unknown_title: "Orrery Remote",
    unknown_body: "Một thiết bị lạ thử kết nối và đã bị từ chối.",
    connected_title: "Orrery Remote",
    vi: true,
};

pub fn remote_text(locale: &str) -> &'static RemoteText {
    if locale == "en" {
        &EN
    } else {
        &VI
    }
}

fn shown_ip(ip: Option<&str>) -> Option<&str> {
    ip.filter(|s| !s.is_empty() && s.len() <= 45 && s.bytes().all(|b| b.is_ascii_hexdigit() || b == b'.' || b == b':'))
}

fn shown_country(c: Option<&str>) -> Option<&str> {
    c.filter(|s| s.len() == 2 && s.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()))
}

pub fn shown_name(name: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| !c.is_control() && !is_hidden(*c) && !matches!(c, '«' | '»' | '“' | '”' | '‘' | '’' | '"' | '\'' | '`'))
        .collect();
    let kept = kept.split_whitespace().collect::<Vec<_>>().join(" ");
    if kept.is_empty() {
        "?".to_owned()
    } else {
        kept
    }
}

impl RemoteText {
    pub fn pair_title(&self, ask: &PairAsk) -> String {
        let name = shown_name(&ask.name);
        let platform = if valid_platform(&ask.platform) { format!("{} ", ask.platform) } else { String::new() };
        let country = shown_country(ask.country.as_deref()).map(|c| format!(" [{c}]")).unwrap_or_default();
        let src = self.pair_source(ask);
        match (self.vi, shown_ip(ask.ip.as_deref())) {
            (true, Some(ip)) => format!("Ghép {src} từ IP {ip}{country}: điện thoại {platform}«{name}»?"),
            (true, None) => format!("Ghép {src} từ IP không rõ{country}: điện thoại {platform}«{name}»?"),
            (false, Some(ip)) => format!("Pair {src} from IP {ip}{country}: {platform}phone “{name}”?"),
            (false, None) => format!("Pair {src} from an unknown IP{country}: {platform}phone “{name}”?"),
        }
    }

    fn pair_source(&self, ask: &PairAsk) -> String {
        let via = if ask.via == "relay" { "relay" } else { "LAN" };
        match (self.vi, ask.source) {
            (true, "usb") => "qua cáp USB".to_owned(),
            (true, "lan") => "qua trang cài".to_owned(),
            (true, "qr") => format!("qua mã QR ({via})"),
            (true, _) => format!("qua {via}"),
            (false, "usb") => "over USB cable".to_owned(),
            (false, "lan") => "via the install page".to_owned(),
            (false, "qr") => format!("via QR code ({via})"),
            (false, _) => format!("via {via}"),
        }
    }

    pub fn connected_body(&self, name: &str) -> String {
        if self.vi {
            format!("«{name}» đã kết nối.")
        } else {
            format!("“{name}” connected.")
        }
    }
}
