use std::path::Path;

use serde_json::Value;
use tauri::{AppHandle, Manager, Webview};

use crate::appearance::zoom_factor;
use crate::capture::{clipboard_png, dust_png, parse_png_data_url, parse_rect};
use crate::guest::{label, Guests};
use crate::native;
use crate::state::{lock, Shell};

pub fn capture_rect(webview: &Webview, raw: &Value) -> Option<Vec<u8>> {
    let rect = parse_rect(raw)?;
    let zoom = zoom_factor(lock(&webview.app_handle().state::<Shell>().look).state.zoom);
    let png = native::capture(webview).ok()?;
    let size = webview.size().ok()?;
    let scale = webview.window().scale_factor().ok().filter(|s| *s > 0.0).unwrap_or(1.0);
    let logical_w = f64::from(size.width) / scale;
    let width = image_width(&png)?;
    if logical_w < 1.0 {
        return None;
    }
    dust_png(&png, rect, zoom, f64::from(width) / logical_w)
}

pub fn base64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn image_width(png: &[u8]) -> Option<u32> {
    (png.len() >= 24).then(|| u32::from_be_bytes([png[16], png[17], png[18], png[19]]))
}

pub fn capture_guest(app: &AppHandle, owner: &str, id: &Value) -> bool {
    let Some(id) = id.as_u64() else { return false };
    if app.state::<Guests>().owner(id).as_deref() != Some(owner) {
        return false;
    }
    let Some(wv) = app.get_webview(&label(id)) else { return false };
    native::capture(&wv).is_ok_and(|png| clipboard_png(&png))
}

pub fn clipboard_data_url(raw: &Value) -> bool {
    parse_png_data_url(raw).is_some_and(|png| clipboard_png(&png))
}

pub fn clipboard_file(raw: &Value) -> bool {
    raw.as_str().is_some_and(|f| crate::capture::clipboard_file(Path::new(f)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_png_width_comes_from_the_header() {
        let mut head = vec![0u8; 24];
        head[16..20].copy_from_slice(&640u32.to_be_bytes());
        assert_eq!(image_width(&head), Some(640));
        assert_eq!(image_width(&[0u8; 10]), None);
    }

    #[test]
    fn bad_clipboard_input_is_refused() {
        assert!(!clipboard_data_url(&json!("data:image/png;base64,AAAA")));
        assert!(!clipboard_file(&json!("relative.png")));
        assert!(!clipboard_file(&json!(3)));
    }
}
