use std::borrow::Cow;
use std::io::Cursor;
use std::path::Path;

use base64::Engine;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, RgbaImage};
use serde_json::Value;

pub const DUST_MAX_PIXELS: f64 = 160_000.0;
pub const MAX_RECT_PIXELS: f64 = 4_000_000.0;
pub const MAX_CLIPBOARD_IMAGE_BYTES: usize = 20 * 1024 * 1024;
const PNG_PREFIX: &str = "data:image/png;base64,";
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const IMAGE_EXT: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn parse_rect(raw: &Value) -> Option<Rect> {
    let n = |k: &str| raw.get(k).and_then(Value::as_f64).filter(|v| v.is_finite());
    Some(Rect { x: n("x")?, y: n("y")?, width: n("width")?, height: n("height")? })
}

pub fn dust_size(width: f64, height: f64, device: (u32, u32)) -> (u32, u32) {
    let w = width.round().min(f64::from(device.0)).max(1.0);
    let h = height.round().min(f64::from(device.1)).max(1.0);
    let k = (DUST_MAX_PIXELS / (w * h)).sqrt().min(1.0);
    ((w * k).round().max(1.0) as u32, (h * k).round().max(1.0) as u32)
}

pub fn crop_box(rect: Rect, zoom: f64, px_per_dip: f64, image: (u32, u32)) -> Option<(u32, u32, u32, u32, f64, f64)> {
    let (dw, dh) = ((rect.width * zoom).round(), (rect.height * zoom).round());
    if dw < 1.0 || dh < 1.0 || dw * dh > MAX_RECT_PIXELS {
        return None;
    }
    let x = ((rect.x * zoom).round().max(0.0) * px_per_dip).round() as i64;
    let y = ((rect.y * zoom).round().max(0.0) * px_per_dip).round() as i64;
    let w = (dw * px_per_dip).round() as i64;
    let h = (dh * px_per_dip).round() as i64;
    let x = x.clamp(0, i64::from(image.0));
    let y = y.clamp(0, i64::from(image.1));
    let w = w.min(i64::from(image.0) - x);
    let h = h.min(i64::from(image.1) - y);
    (w >= 1 && h >= 1).then_some((x as u32, y as u32, w as u32, h as u32, dw, dh))
}

fn encode(img: &DynamicImage) -> Option<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

pub fn dust_png(png: &[u8], rect: Rect, zoom: f64, px_per_dip: f64) -> Option<Vec<u8>> {
    let full = image::load_from_memory_with_format(png, ImageFormat::Png).ok()?;
    let (x, y, w, h, dw, dh) = crop_box(rect, zoom, px_per_dip, (full.width(), full.height()))?;
    let part = full.crop_imm(x, y, w, h);
    let (tw, th) = dust_size(dw, dh, (w, h));
    let out = if tw < w || th < h { part.resize_exact(tw, th, FilterType::Triangle) } else { part };
    encode(&out)
}

pub fn parse_png_data_url(value: &Value) -> Option<Vec<u8>> {
    let b64 = value.as_str()?.strip_prefix(PNG_PREFIX)?;
    if b64.is_empty() || b64.len() > MAX_CLIPBOARD_IMAGE_BYTES.div_ceil(3) * 4 {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    (bytes.len() <= MAX_CLIPBOARD_IMAGE_BYTES && bytes.starts_with(&PNG_SIGNATURE)).then_some(bytes)
}

pub fn image_file_ok(file: &Path) -> bool {
    let ext = file.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    file.is_absolute() && IMAGE_EXT.contains(&ext.as_str()) && std::fs::metadata(file).is_ok_and(|m| m.is_file() && m.len() as usize <= MAX_CLIPBOARD_IMAGE_BYTES)
}

fn write_rgba(img: RgbaImage) -> bool {
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return false;
    }
    let data = arboard::ImageData { width: width as usize, height: height as usize, bytes: Cow::Owned(img.into_raw()) };
    arboard::Clipboard::new().and_then(|mut c| c.set_image(data)).is_ok()
}

pub fn clipboard_png(png: &[u8]) -> bool {
    image::load_from_memory(png).is_ok_and(|img| write_rgba(img.to_rgba8()))
}

pub fn clipboard_file(file: &Path) -> bool {
    image_file_ok(file) && image::open(file).is_ok_and(|img| write_rgba(img.to_rgba8()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn png(w: u32, h: u32) -> Vec<u8> {
        encode(&DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, image::Rgba([10, 20, 30, 255])))).unwrap()
    }

    #[test]
    fn dust_size_matches_the_electron_rule() {
        assert_eq!(dust_size(100.0, 50.0, (200, 100)), (100, 50));
        assert_eq!(dust_size(800.0, 800.0, (1600, 1600)), (400, 400));
        assert_eq!(dust_size(300.0, 300.0, (100, 100)), (100, 100));
        assert_eq!(dust_size(0.2, 0.2, (10, 10)), (1, 1));
    }

    #[test]
    fn rects_are_read_from_numbers_only() {
        assert_eq!(parse_rect(&json!({"x": 1, "y": 2.5, "width": 3, "height": 4})), Some(Rect { x: 1.0, y: 2.5, width: 3.0, height: 4.0 }));
        assert_eq!(parse_rect(&json!({"x": "1", "y": 2, "width": 3, "height": 4})), None);
        assert_eq!(parse_rect(&json!({"x": 1, "y": 2, "width": 3})), None);
    }

    #[test]
    fn the_crop_box_follows_zoom_and_device_pixels_and_stays_inside() {
        let r = Rect { x: 10.0, y: 20.0, width: 100.0, height: 50.0 };
        assert_eq!(crop_box(r, 1.0, 1.5, (1000, 1000)), Some((15, 30, 150, 75, 100.0, 50.0)));
        assert_eq!(crop_box(r, 1.2, 1.0, (1000, 1000)), Some((12, 24, 120, 60, 120.0, 60.0)));
        assert_eq!(crop_box(Rect { x: 950.0, ..r }, 1.0, 1.0, (1000, 1000)), Some((950, 20, 50, 50, 100.0, 50.0)));
        assert_eq!(crop_box(Rect { width: 0.2, ..r }, 1.0, 1.0, (1000, 1000)), None);
        assert_eq!(crop_box(Rect { width: 3000.0, height: 3000.0, ..r }, 1.0, 1.0, (5000, 5000)), None);
        assert_eq!(crop_box(Rect { x: 2000.0, ..r }, 1.0, 1.0, (1000, 1000)), None);
    }

    #[test]
    fn a_dust_shot_is_cropped_then_shrunk_to_css_size() {
        let shot = png(400, 300);
        let out = dust_png(&shot, Rect { x: 10.0, y: 10.0, width: 100.0, height: 60.0 }, 1.0, 2.0).unwrap();
        let img = image::load_from_memory(&out).unwrap();
        assert_eq!((img.width(), img.height()), (100, 60));
    }

    #[test]
    fn png_data_urls_must_be_real_png() {
        let shot = png(2, 2);
        let url = format!("{PNG_PREFIX}{}", base64::engine::general_purpose::STANDARD.encode(&shot));
        assert_eq!(parse_png_data_url(&json!(url)), Some(shot));
        assert_eq!(parse_png_data_url(&json!("data:image/png;base64,aGVsbG8=")), None);
        assert_eq!(parse_png_data_url(&json!("data:image/jpeg;base64,aGVsbG8=")), None);
        assert_eq!(parse_png_data_url(&json!(5)), None);
    }

    #[test]
    fn image_files_need_an_absolute_image_path() {
        assert!(!image_file_ok(Path::new("relative.png")));
        let dir = std::env::temp_dir();
        let txt = dir.join(format!("orrery-shell-cap-{}.txt", std::process::id()));
        std::fs::write(&txt, "x").unwrap();
        assert!(!image_file_ok(&txt));
        let pic = dir.join(format!("orrery-shell-cap-{}.png", std::process::id()));
        std::fs::write(&pic, png(1, 1)).unwrap();
        assert!(image_file_ok(&pic));
        let _ = std::fs::remove_file(txt);
        let _ = std::fs::remove_file(pic);
    }
}
