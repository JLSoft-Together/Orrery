use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

const RELOGIN_EN: (&str, &str) = (
    "Telegram: sign in again",
    "Orrery now talks to Telegram in a new way. Open Settings \u{203a} Telegram and sign in to your account again to list topics.",
);
const RELOGIN_VI: (&str, &str) = (
    "Telegram: c\u{1ea7}n \u{111}\u{103}ng nh\u{1ead}p l\u{1ea1}i",
    "Orrery \u{111}\u{1ed5}i sang c\u{e1}ch k\u{1ebf}t n\u{1ed1}i Telegram m\u{1edb}i. M\u{1edf} Settings \u{203a} Telegram v\u{e0} \u{111}\u{103}ng nh\u{1ead}p l\u{1ea1}i t\u{e0}i kho\u{1ea3}n \u{111}\u{1ec3} \u{111}\u{1ecd}c danh s\u{e1}ch topic.",
);

pub fn relogin_text(locale: &str) -> (&'static str, &'static str) {
    if locale == "vi" {
        RELOGIN_VI
    } else {
        RELOGIN_EN
    }
}

pub fn telegram_notice(app: &AppHandle, locale: &str, p: &Value) -> bool {
    if p.get("kind").and_then(Value::as_str) != Some("relogin") {
        return false;
    }
    let (title, body) = relogin_text(locale);
    app.notification().builder().title(title).body(body).show().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_relogin_text_follows_the_locale_and_defaults_to_english() {
        assert_eq!(relogin_text("vi").0, "Telegram: cần đăng nhập lại");
        assert_eq!(relogin_text("en").0, "Telegram: sign in again");
        assert_eq!(relogin_text("fr").0, "Telegram: sign in again");
        assert!(relogin_text("vi").1.contains("Settings › Telegram"));
        assert!(relogin_text("en").1.contains("Settings › Telegram"));
    }
}
