use super::config::now_secs;
use super::ctx::Ctx;
use super::text::shown_name;

pub struct AlertText {
    pub title: &'static str,
    pub body: String,
    pub tg: String,
}

fn vi(ctx: &Ctx) -> bool {
    ctx.host.welcome().locale != "en"
}

fn place(ip: Option<&str>, country: Option<&str>, via: &str, vi: bool) -> String {
    let ip = ip.unwrap_or(if vi { "IP không rõ" } else { "unknown IP" });
    let country = country.map(|c| format!(" [{c}]")).unwrap_or_default();
    let via = if via == "relay" { "relay" } else { "LAN" };
    if vi {
        format!("{ip}{country} qua {via}")
    } else {
        format!("{ip}{country} via {via}")
    }
}

fn route(country: Option<&str>, via: &str, vi: bool) -> String {
    let country = country.map(|c| format!(" [{c}]")).unwrap_or_default();
    let via = if via == "relay" { "relay" } else { "LAN" };
    if vi {
        format!("qua {via}{country}")
    } else {
        format!("via {via}{country}")
    }
}

fn hint(vi: bool) -> &'static str {
    if vi {
        "Xem chi tiết trong Cài đặt → Điều khiển từ xa trên máy tính."
    } else {
        "See the details in Settings → Remote on the computer."
    }
}

pub fn paired_text(vi: bool, name: &str, ip: Option<&str>, country: Option<&str>, via: &str) -> AlertText {
    let name = shown_name(name);
    let at = place(ip, country, via, vi);
    let r = route(country, via, vi);
    if vi {
        AlertText {
            title: "Orrery Remote: thiết bị mới",
            body: format!("«{name}» vừa được ghép từ {at}. Không phải bạn? Thu hồi ngay trong Cài đặt → Điều khiển từ xa."),
            tg: format!("Orrery Remote: một thiết bị mới vừa được ghép {r}. Không phải bạn? Thu hồi ngay. {}", hint(vi)),
        }
    } else {
        AlertText {
            title: "Orrery Remote: new device",
            body: format!("“{name}” was just paired from {at}. Not you? Revoke it now in Settings → Remote."),
            tg: format!("Orrery Remote: a new device was just paired {r}. Not you? Revoke it now. {}", hint(vi)),
        }
    }
}

pub fn blocked_text(vi: bool, key: &str, source: &str, missed: u32) -> AlertText {
    let more = match (missed, vi) {
        (0, _) => String::new(),
        (n, true) => format!(" (và {n} IP khác trong giờ qua)"),
        (n, false) => format!(" (and {n} more in the last hour)"),
    };
    let src = if source == "relay" { "relay" } else { "LAN" };
    if vi {
        AlertText {
            title: "Orrery Remote: đã chặn IP",
            body: format!("IP {key} bị chặn trên {src} vì thử sai nhiều lần{more}. Xem trong Cài đặt → Điều khiển từ xa."),
            tg: format!("Orrery Remote: đã chặn 1 IP trên {src} vì thử sai nhiều lần{more}. {}", hint(vi)),
        }
    } else {
        AlertText {
            title: "Orrery Remote: IP blocked",
            body: format!("IP {key} was blocked on the {src} after repeated failures{more}. See Settings → Remote."),
            tg: format!("Orrery Remote: an IP was blocked on the {src} after repeated failures{more}. {}", hint(vi)),
        }
    }
}

pub fn place_text(vi: bool, name: &str, ip: Option<&str>, country: Option<&str>) -> AlertText {
    let name = shown_name(name);
    let at = place(ip, country, "relay", vi);
    let c = country.unwrap_or("?");
    if vi {
        AlertText {
            title: "Orrery Remote: quốc gia lạ",
            body: format!("«{name}» vừa nối từ quốc gia chưa thấy: {at}."),
            tg: format!("Orrery Remote: một điện thoại đã ghép vừa nối qua relay từ quốc gia chưa thấy [{c}]. {}", hint(vi)),
        }
    } else {
        AlertText {
            title: "Orrery Remote: new country",
            body: format!("“{name}” connected from a country not seen before: {at}."),
            tg: format!("Orrery Remote: a paired phone connected over the relay from a country not seen before [{c}]. {}", hint(vi)),
        }
    }
}

impl Ctx {
    fn raise(&self, text: &AlertText, except: Option<&str>) {
        self.host.notify(text.title, &text.body);
        self.host.telegram(&text.tg);
        self.push_security(except);
    }

    pub fn alert_paired(&self, device: &str, name: &str, ip: Option<&str>, country: Option<&str>, via: &str) {
        self.raise(&paired_text(vi(self), name, ip, country, via), Some(device));
    }

    pub fn alert_blocked(&self, key: &str, source: &str) {
        let Some(missed) = self.guard.alert_gate(now_secs()) else { return };
        self.raise(&blocked_text(vi(self), key, source, missed), None);
    }

    pub fn note_place(&self, device: &str, ip: Option<&str>, country: Option<&str>, alert: bool) {
        let (fresh, name) = {
            let mut devices = crate::state::lock(&self.devices);
            let fresh = devices.note_place(device, country);
            (fresh, devices.find(device).map(|d| d.display_name().to_owned()).unwrap_or_default())
        };
        if alert && fresh && self.guard.place_gate(device, now_secs()) {
            self.raise(&place_text(vi(self), &name, ip, country), Some(device));
        }
    }
}
