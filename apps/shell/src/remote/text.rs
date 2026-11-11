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

impl RemoteText {
    pub fn pair_title(&self, name: &str) -> String {
        if self.vi {
            format!("Cho phép «{name}» điều khiển Orrery?")
        } else {
            format!("Allow “{name}” to control Orrery?")
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
