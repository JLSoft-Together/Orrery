#[cfg(windows)]
use tauri::image::Image;
use tauri::{AppHandle, Manager, UserAttentionType};

use crate::state::MAIN;

const SIZE: u32 = 32;
const GLYPH_W: u32 = 3;
const GLYPH_H: u32 = 5;
const SCALE: u32 = 4;
const GAP: u32 = 2;
const FILL: [u8; 4] = [0xe5, 0x39, 0x35, 0xff];
const INK: [u8; 4] = [0xff, 0xff, 0xff, 0xff];

const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b001, 0b001, 0b001],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];
const PLUS: [u8; 5] = [0b000, 0b010, 0b111, 0b010, 0b000];

pub fn label(count: u32) -> Option<String> {
    match count {
        0 => None,
        1..=9 => Some(count.to_string()),
        _ => Some("9+".into()),
    }
}

fn glyphs(label: &str) -> Vec<[u8; 5]> {
    label.chars().map(|c| if c == '+' { PLUS } else { DIGITS[c.to_digit(10).unwrap_or(0) as usize] }).collect()
}

pub fn icon_rgba(label: &str) -> Vec<u8> {
    let mut px = vec![0u8; (SIZE * SIZE * 4) as usize];
    let r = SIZE as f32 / 2.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (dx, dy) = (x as f32 + 0.5 - r, y as f32 + 0.5 - r);
            if dx * dx + dy * dy <= r * r {
                let i = ((y * SIZE + x) * 4) as usize;
                px[i..i + 4].copy_from_slice(&FILL);
            }
        }
    }
    let rows = glyphs(label);
    let n = rows.len() as u32;
    let scale = if n > 1 { 3 } else { SCALE };
    let width = n * GLYPH_W * scale + n.saturating_sub(1) * (GAP / 2);
    let left = (SIZE - width) / 2;
    let top = (SIZE - GLYPH_H * scale) / 2;
    for (gi, glyph) in rows.iter().enumerate() {
        let gx = left + gi as u32 * (GLYPH_W * scale + GAP / 2);
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..GLYPH_W {
                if bits >> (GLYPH_W - 1 - col) & 1 == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let (x, y) = (gx + col * scale + sx, top + row as u32 * scale + sy);
                        let i = ((y * SIZE + x) * 4) as usize;
                        px[i..i + 4].copy_from_slice(&INK);
                    }
                }
            }
        }
    }
    px
}

pub fn should_flash(flash: bool, count: u32, focused: bool) -> bool {
    flash && count > 0 && !focused
}

pub fn apply(app: &AppHandle, count: u32, flash: bool) {
    let Some(window) = app.get_webview_window(MAIN) else { return };
    let text = label(count);
    #[cfg(windows)]
    {
        let icon = text.as_deref().map(|t| Image::new_owned(icon_rgba(t), SIZE, SIZE));
        let _ = window.set_overlay_icon(icon);
    }
    #[cfg(target_os = "macos")]
    {
        let _ = window.set_badge_label(text.clone());
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = window.set_badge_count(if count == 0 { None } else { Some(i64::from(count)) });
    }
    if should_flash(flash, count, window.is_focused().unwrap_or(true)) {
        let _ = window.request_user_attention(Some(UserAttentionType::Informational));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_cap_at_nine_plus() {
        assert_eq!(label(0), None);
        assert_eq!(label(1).as_deref(), Some("1"));
        assert_eq!(label(9).as_deref(), Some("9"));
        assert_eq!(label(10).as_deref(), Some("9+"));
        assert_eq!(label(400).as_deref(), Some("9+"));
    }

    #[test]
    fn the_icon_is_a_red_disc_with_white_ink() {
        for text in ["1", "7", "9+"] {
            let px = icon_rgba(text);
            assert_eq!(px.len(), (SIZE * SIZE * 4) as usize);
            assert_eq!(&px[0..4], &[0, 0, 0, 0]);
            let at = |x: u32, y: u32| &px[((y * SIZE + x) * 4) as usize..][..4];
            assert_eq!(at(3, 16), &FILL);
            assert!(px.chunks(4).any(|p| p == INK), "{text}");
        }
    }

    #[test]
    fn only_a_new_item_in_an_unfocused_window_flashes() {
        assert!(should_flash(true, 2, false));
        assert!(!should_flash(true, 2, true));
        assert!(!should_flash(false, 2, false));
        assert!(!should_flash(true, 0, false));
    }
}
