# RESEARCH v108: bảng màu dịu mắt (2026-11-20)

Mockup: `.planning/site/public/mau/index.html` (data: `mau/palettes.js`, cùng nguồn với file này). Chủ nhân chọn trên trang, bấm "Copy lựa chọn" rồi dán lại.

Mục tiêu: màu tổng thể basic, hài hoà, đỡ mỏi mắt; thêm chế độ "Ngẫu nhiên" tự trộn bảng màu mới lúc mở app; bỏ theme trùng.

Vòng 2 (chủ nhân 20/11: "anh muốn theme nó basic hơn chút"): 5 bảng dùng chung 1 nền xám trung tính, chỉ khác accent.

## Nguyên tắc chung

- Nền xám trung tính tuyệt đối (R = G = B), không ám màu. Light gần trắng (canvas `#f7f7f7`, surface `#ffffff`), dark kiểu editor (canvas `#1e1e1e`, surface `#252525`).
- Phẳng: không gradient, không glow, không shadow trên pane; border 1px `line` tối thiểu.
- Terminal well: dark = đen trung tính `#121212`, light = trắng `#ffffff` (viền `line`).
- 1 accent duy nhất mỗi bảng, chỉ cho nút chính, chọn, focus, tab đang mở (COLOR-SYSTEM rule 4).
- Status giữ hue cố định: working xanh dương, waiting hổ phách, idle xám, done xanh lá, error đỏ. Status luôn glyph + chữ + màu. Agent: Claude cam, Codex tím, shell xanh mòng két.
- Soft tint = `color-mix(in srgb, <màu> N%, surface)`, N: working 12, waiting 14, error 12, done 14, diff-add 14, diff-del 12, accent-soft 14.
- Contrast (WCAG): chữ ≥ 4.5:1 trên canvas / side / surface / raised và trên soft tint của chính nó; on-accent trên accent ≥ 4.5; glyph (accent, agent) ≥ 3 trên surface; chữ trên well ≥ 4.5. 10 biến thể đều đạt.
- Lục trùng hue với done, Tím gần Codex, Xanh gần working, Xanh ngọc gần shell: chấp nhận cho preset (chủ nhân chọn tên), vì status / agent luôn có glyph + chữ. Bộ trộn ngẫu nhiên thì tránh hẳn (xem dưới).

## Map sang token app

| Mockup | Token `--ccm-*` |
|---|---|
| side | side / panel-bg |
| agent-shell | agent-shell-mark |
| well-accent | ansi prompt / accent ink trên well |
| well-green / red / blue / yellow | ansi green / red / blue / yellow, diff-add-well / diff-del-well |
| còn lại | trùng tên |

## Xanh (`blue`): Xanh dương chuẩn

| Token | Light | Dark |
|---|---|---|
| canvas | `#f7f7f7` | `#1e1e1e` |
| side | `#f2f2f2` | `#181818` |
| surface | `#ffffff` | `#252525` |
| raised | `#fafafa` | `#2b2b2b` |
| control | `#ffffff` | `#333333` |
| line | `#e5e5e5` | `#333333` |
| line-strong | `#d4d4d4` | `#444444` |
| ink | `#1f1f1f` | `#e6e6e6` |
| ink-muted | `#525252` | `#b0b0b0` |
| ink-faint | `#636363` | `#9a9a9a` |
| working | `#1f5fbf` | `#7fb0ff` |
| waiting | `#7a5a00` | `#e5c07b` |
| idle | `#636363` | `#9a9a9a` |
| done | `#1b6e37` | `#6fcf8f` |
| error | `#c02a2a` | `#f28b82` |
| agent-claude | `#b85a35` | `#d9825f` |
| agent-codex | `#6750b8` | `#a89cf0` |
| agent-shell | `#17786f` | `#5cc0b4` |
| diff-add | `#1b6e37` | `#6fcf8f` |
| diff-del | `#c02a2a` | `#f28b82` |
| well | `#ffffff` | `#121212` |
| well-ink | `#1f1f1f` | `#e6e6e6` |
| well-dim | `#636363` | `#8c8c8c` |
| well-green | `#1b6e37` | `#6fcf8f` |
| well-red | `#c02a2a` | `#f28b82` |
| well-blue | `#1f5fbf` | `#8ab4f8` |
| well-yellow | `#7a5a00` | `#e5c07b` |
| accent | `#0a66d6` | `#4c9aff` |
| accent-ink | `#0758bd` | `#7fb3ff` |
| on-accent | `#ffffff` | `#0a1a33` |
| well-accent | `#0758bd` | `#7fb3ff` |

Contrast min chữ: light **4.80**, dark **5.03** (cần ≥ 4.5).

| Chữ / glyph | Trên | Light | Dark | Cần |
|---|---|---|---|---|
| ink | side | 14.72 | 14.23 | 4.5 |
| ink | surface | 16.48 | 12.28 | 4.5 |
| ink | raised | 15.79 | 11.34 | 4.5 |
| ink-muted | side | 6.98 | 8.19 | 4.5 |
| ink-muted | surface | 7.81 | 7.07 | 4.5 |
| ink-muted | raised | 7.49 | 6.53 | 4.5 |
| ink-faint | side | 5.37 | 6.31 | 4.5 |
| ink-faint | surface | 6.01 | 5.45 | 4.5 |
| ink-faint | raised | 5.76 | 5.03 | 4.5 |
| accent-ink | side | 5.97 | 8.28 | 4.5 |
| accent-ink | surface | 6.69 | 7.15 | 4.5 |
| accent-ink | raised | 6.41 | 6.60 | 4.5 |
| working | side | 5.44 | 8.08 | 4.5 |
| working | surface | 6.09 | 6.98 | 4.5 |
| working | raised | 5.84 | 6.44 | 4.5 |
| waiting | side | 5.70 | 10.28 | 4.5 |
| waiting | surface | 6.38 | 8.87 | 4.5 |
| waiting | raised | 6.12 | 8.20 | 4.5 |
| idle | side | 5.37 | 6.31 | 4.5 |
| idle | surface | 6.01 | 5.45 | 4.5 |
| idle | raised | 5.76 | 5.03 | 4.5 |
| done | side | 5.63 | 9.30 | 4.5 |
| done | surface | 6.30 | 8.03 | 4.5 |
| done | raised | 6.04 | 7.42 | 4.5 |
| error | side | 5.20 | 7.43 | 4.5 |
| error | surface | 5.82 | 6.42 | 4.5 |
| error | raised | 5.58 | 5.93 | 4.5 |
| diff-add | side | 5.63 | 9.30 | 4.5 |
| diff-add | surface | 6.30 | 8.03 | 4.5 |
| diff-add | raised | 6.04 | 7.42 | 4.5 |
| diff-del | side | 5.20 | 7.43 | 4.5 |
| diff-del | surface | 5.82 | 6.42 | 4.5 |
| diff-del | raised | 5.58 | 5.93 | 4.5 |
| working | working-soft | 5.12 | 5.54 | 4.5 |
| waiting | waiting-soft | 5.21 | 6.44 | 4.5 |
| error | error-soft | 4.80 | 5.21 | 4.5 |
| done | done-soft | 5.14 | 5.98 | 4.5 |
| diff-add | diff-add-soft | 5.14 | 5.98 | 4.5 |
| diff-del | diff-del-soft | 4.80 | 5.21 | 4.5 |
| accent-ink | accent-ink-soft | 5.48 | 5.79 | 4.5 |
| on-accent | accent | 5.41 | 6.10 | 4.5 |
| ink | control | 16.48 | 10.12 | 4.5 |
| accent | surface | 5.41 | 5.38 | 3 |
| agent-claude | surface | 4.61 | 5.33 | 3 |
| agent-codex | surface | 6.15 | 6.33 | 3 |
| agent-shell | surface | 5.31 | 7.05 | 3 |
| well-ink | well | 16.48 | 15.01 | 4.5 |
| well-dim | well | 6.01 | 5.57 | 4.5 |
| well-accent | well | 6.69 | 8.74 | 4.5 |
| well-green | well | 6.30 | 9.81 | 4.5 |
| well-red | well | 5.82 | 7.84 | 4.5 |
| well-blue | well | 6.09 | 8.89 | 4.5 |
| well-yellow | well | 6.38 | 10.85 | 4.5 |

## Đen trắng (`mono`): Không màu nhấn

| Token | Light | Dark |
|---|---|---|
| canvas | `#f7f7f7` | `#1e1e1e` |
| side | `#f2f2f2` | `#181818` |
| surface | `#ffffff` | `#252525` |
| raised | `#fafafa` | `#2b2b2b` |
| control | `#ffffff` | `#333333` |
| line | `#e5e5e5` | `#333333` |
| line-strong | `#d4d4d4` | `#444444` |
| ink | `#1f1f1f` | `#e6e6e6` |
| ink-muted | `#525252` | `#b0b0b0` |
| ink-faint | `#636363` | `#9a9a9a` |
| working | `#1f5fbf` | `#7fb0ff` |
| waiting | `#7a5a00` | `#e5c07b` |
| idle | `#636363` | `#9a9a9a` |
| done | `#1b6e37` | `#6fcf8f` |
| error | `#c02a2a` | `#f28b82` |
| agent-claude | `#b85a35` | `#d9825f` |
| agent-codex | `#6750b8` | `#a89cf0` |
| agent-shell | `#17786f` | `#5cc0b4` |
| diff-add | `#1b6e37` | `#6fcf8f` |
| diff-del | `#c02a2a` | `#f28b82` |
| well | `#ffffff` | `#121212` |
| well-ink | `#1f1f1f` | `#e6e6e6` |
| well-dim | `#636363` | `#8c8c8c` |
| well-green | `#1b6e37` | `#6fcf8f` |
| well-red | `#c02a2a` | `#f28b82` |
| well-blue | `#1f5fbf` | `#8ab4f8` |
| well-yellow | `#7a5a00` | `#e5c07b` |
| accent | `#262626` | `#e6e6e6` |
| accent-ink | `#262626` | `#f0f0f0` |
| on-accent | `#ffffff` | `#1e1e1e` |
| well-accent | `#262626` | `#f0f0f0` |

Contrast min chữ: light **4.80**, dark **5.03** (cần ≥ 4.5).

| Chữ / glyph | Trên | Light | Dark | Cần |
|---|---|---|---|---|
| ink | side | 14.72 | 14.23 | 4.5 |
| ink | surface | 16.48 | 12.28 | 4.5 |
| ink | raised | 15.79 | 11.34 | 4.5 |
| ink-muted | side | 6.98 | 8.19 | 4.5 |
| ink-muted | surface | 7.81 | 7.07 | 4.5 |
| ink-muted | raised | 7.49 | 6.53 | 4.5 |
| ink-faint | side | 5.37 | 6.31 | 4.5 |
| ink-faint | surface | 6.01 | 5.45 | 4.5 |
| ink-faint | raised | 5.76 | 5.03 | 4.5 |
| accent-ink | side | 13.52 | 15.58 | 4.5 |
| accent-ink | surface | 15.13 | 13.45 | 4.5 |
| accent-ink | raised | 14.50 | 12.42 | 4.5 |
| working | side | 5.44 | 8.08 | 4.5 |
| working | surface | 6.09 | 6.98 | 4.5 |
| working | raised | 5.84 | 6.44 | 4.5 |
| waiting | side | 5.70 | 10.28 | 4.5 |
| waiting | surface | 6.38 | 8.87 | 4.5 |
| waiting | raised | 6.12 | 8.20 | 4.5 |
| idle | side | 5.37 | 6.31 | 4.5 |
| idle | surface | 6.01 | 5.45 | 4.5 |
| idle | raised | 5.76 | 5.03 | 4.5 |
| done | side | 5.63 | 9.30 | 4.5 |
| done | surface | 6.30 | 8.03 | 4.5 |
| done | raised | 6.04 | 7.42 | 4.5 |
| error | side | 5.20 | 7.43 | 4.5 |
| error | surface | 5.82 | 6.42 | 4.5 |
| error | raised | 5.58 | 5.93 | 4.5 |
| diff-add | side | 5.63 | 9.30 | 4.5 |
| diff-add | surface | 6.30 | 8.03 | 4.5 |
| diff-add | raised | 6.04 | 7.42 | 4.5 |
| diff-del | side | 5.20 | 7.43 | 4.5 |
| diff-del | surface | 5.82 | 6.42 | 4.5 |
| diff-del | raised | 5.58 | 5.93 | 4.5 |
| working | working-soft | 5.12 | 5.54 | 4.5 |
| waiting | waiting-soft | 5.21 | 6.44 | 4.5 |
| error | error-soft | 4.80 | 5.21 | 4.5 |
| done | done-soft | 5.14 | 5.98 | 4.5 |
| diff-add | diff-add-soft | 5.14 | 5.98 | 4.5 |
| diff-del | diff-del-soft | 4.80 | 5.21 | 4.5 |
| accent-ink | accent-ink-soft | 11.57 | 9.10 | 4.5 |
| on-accent | accent | 15.13 | 13.36 | 4.5 |
| ink | control | 16.48 | 10.12 | 4.5 |
| accent | surface | 15.13 | 12.28 | 3 |
| agent-claude | surface | 4.61 | 5.33 | 3 |
| agent-codex | surface | 6.15 | 6.33 | 3 |
| agent-shell | surface | 5.31 | 7.05 | 3 |
| well-ink | well | 16.48 | 15.01 | 4.5 |
| well-dim | well | 6.01 | 5.57 | 4.5 |
| well-accent | well | 15.13 | 16.44 | 4.5 |
| well-green | well | 6.30 | 9.81 | 4.5 |
| well-red | well | 5.82 | 7.84 | 4.5 |
| well-blue | well | 6.09 | 8.89 | 4.5 |
| well-yellow | well | 6.38 | 10.85 | 4.5 |

## Lục (`green`): Xanh lá

| Token | Light | Dark |
|---|---|---|
| canvas | `#f7f7f7` | `#1e1e1e` |
| side | `#f2f2f2` | `#181818` |
| surface | `#ffffff` | `#252525` |
| raised | `#fafafa` | `#2b2b2b` |
| control | `#ffffff` | `#333333` |
| line | `#e5e5e5` | `#333333` |
| line-strong | `#d4d4d4` | `#444444` |
| ink | `#1f1f1f` | `#e6e6e6` |
| ink-muted | `#525252` | `#b0b0b0` |
| ink-faint | `#636363` | `#9a9a9a` |
| working | `#1f5fbf` | `#7fb0ff` |
| waiting | `#7a5a00` | `#e5c07b` |
| idle | `#636363` | `#9a9a9a` |
| done | `#1b6e37` | `#6fcf8f` |
| error | `#c02a2a` | `#f28b82` |
| agent-claude | `#b85a35` | `#d9825f` |
| agent-codex | `#6750b8` | `#a89cf0` |
| agent-shell | `#17786f` | `#5cc0b4` |
| diff-add | `#1b6e37` | `#6fcf8f` |
| diff-del | `#c02a2a` | `#f28b82` |
| well | `#ffffff` | `#121212` |
| well-ink | `#1f1f1f` | `#e6e6e6` |
| well-dim | `#636363` | `#8c8c8c` |
| well-green | `#1b6e37` | `#6fcf8f` |
| well-red | `#c02a2a` | `#f28b82` |
| well-blue | `#1f5fbf` | `#8ab4f8` |
| well-yellow | `#7a5a00` | `#e5c07b` |
| accent | `#1f7a44` | `#5cc185` |
| accent-ink | `#1a6a3b` | `#7dd3a0` |
| on-accent | `#ffffff` | `#0c2416` |
| well-accent | `#1a6a3b` | `#7dd3a0` |

Contrast min chữ: light **4.80**, dark **5.03** (cần ≥ 4.5).

| Chữ / glyph | Trên | Light | Dark | Cần |
|---|---|---|---|---|
| ink | side | 14.72 | 14.23 | 4.5 |
| ink | surface | 16.48 | 12.28 | 4.5 |
| ink | raised | 15.79 | 11.34 | 4.5 |
| ink-muted | side | 6.98 | 8.19 | 4.5 |
| ink-muted | surface | 7.81 | 7.07 | 4.5 |
| ink-muted | raised | 7.49 | 6.53 | 4.5 |
| ink-faint | side | 5.37 | 6.31 | 4.5 |
| ink-faint | surface | 6.01 | 5.45 | 4.5 |
| ink-faint | raised | 5.76 | 5.03 | 4.5 |
| accent-ink | side | 5.92 | 9.89 | 4.5 |
| accent-ink | surface | 6.63 | 8.54 | 4.5 |
| accent-ink | raised | 6.35 | 7.89 | 4.5 |
| working | side | 5.44 | 8.08 | 4.5 |
| working | surface | 6.09 | 6.98 | 4.5 |
| working | raised | 5.84 | 6.44 | 4.5 |
| waiting | side | 5.70 | 10.28 | 4.5 |
| waiting | surface | 6.38 | 8.87 | 4.5 |
| waiting | raised | 6.12 | 8.20 | 4.5 |
| idle | side | 5.37 | 6.31 | 4.5 |
| idle | surface | 6.01 | 5.45 | 4.5 |
| idle | raised | 5.76 | 5.03 | 4.5 |
| done | side | 5.63 | 9.30 | 4.5 |
| done | surface | 6.30 | 8.03 | 4.5 |
| done | raised | 6.04 | 7.42 | 4.5 |
| error | side | 5.20 | 7.43 | 4.5 |
| error | surface | 5.82 | 6.42 | 4.5 |
| error | raised | 5.58 | 5.93 | 4.5 |
| diff-add | side | 5.63 | 9.30 | 4.5 |
| diff-add | surface | 6.30 | 8.03 | 4.5 |
| diff-add | raised | 6.04 | 7.42 | 4.5 |
| diff-del | side | 5.20 | 7.43 | 4.5 |
| diff-del | surface | 5.82 | 6.42 | 4.5 |
| diff-del | raised | 5.58 | 5.93 | 4.5 |
| working | working-soft | 5.12 | 5.54 | 4.5 |
| waiting | waiting-soft | 5.21 | 6.44 | 4.5 |
| error | error-soft | 4.80 | 5.21 | 4.5 |
| done | done-soft | 5.14 | 5.98 | 4.5 |
| diff-add | diff-add-soft | 5.14 | 5.98 | 4.5 |
| diff-del | diff-del-soft | 4.80 | 5.21 | 4.5 |
| accent-ink | accent-ink-soft | 5.46 | 6.56 | 4.5 |
| on-accent | accent | 5.35 | 7.36 | 4.5 |
| ink | control | 16.48 | 10.12 | 4.5 |
| accent | surface | 5.35 | 6.88 | 3 |
| agent-claude | surface | 4.61 | 5.33 | 3 |
| agent-codex | surface | 6.15 | 6.33 | 3 |
| agent-shell | surface | 5.31 | 7.05 | 3 |
| well-ink | well | 16.48 | 15.01 | 4.5 |
| well-dim | well | 6.01 | 5.57 | 4.5 |
| well-accent | well | 6.63 | 10.43 | 4.5 |
| well-green | well | 6.30 | 9.81 | 4.5 |
| well-red | well | 5.82 | 7.84 | 4.5 |
| well-blue | well | 6.09 | 8.89 | 4.5 |
| well-yellow | well | 6.38 | 10.85 | 4.5 |

## Tím (`violet`): Tím xanh

| Token | Light | Dark |
|---|---|---|
| canvas | `#f7f7f7` | `#1e1e1e` |
| side | `#f2f2f2` | `#181818` |
| surface | `#ffffff` | `#252525` |
| raised | `#fafafa` | `#2b2b2b` |
| control | `#ffffff` | `#333333` |
| line | `#e5e5e5` | `#333333` |
| line-strong | `#d4d4d4` | `#444444` |
| ink | `#1f1f1f` | `#e6e6e6` |
| ink-muted | `#525252` | `#b0b0b0` |
| ink-faint | `#636363` | `#9a9a9a` |
| working | `#1f5fbf` | `#7fb0ff` |
| waiting | `#7a5a00` | `#e5c07b` |
| idle | `#636363` | `#9a9a9a` |
| done | `#1b6e37` | `#6fcf8f` |
| error | `#c02a2a` | `#f28b82` |
| agent-claude | `#b85a35` | `#d9825f` |
| agent-codex | `#6750b8` | `#a89cf0` |
| agent-shell | `#17786f` | `#5cc0b4` |
| diff-add | `#1b6e37` | `#6fcf8f` |
| diff-del | `#c02a2a` | `#f28b82` |
| well | `#ffffff` | `#121212` |
| well-ink | `#1f1f1f` | `#e6e6e6` |
| well-dim | `#636363` | `#8c8c8c` |
| well-green | `#1b6e37` | `#6fcf8f` |
| well-red | `#c02a2a` | `#f28b82` |
| well-blue | `#1f5fbf` | `#8ab4f8` |
| well-yellow | `#7a5a00` | `#e5c07b` |
| accent | `#5b4fcf` | `#9d94f5` |
| accent-ink | `#4f43bd` | `#b3abff` |
| on-accent | `#ffffff` | `#191536` |
| well-accent | `#4f43bd` | `#b3abff` |

Contrast min chữ: light **4.80**, dark **5.03** (cần ≥ 4.5).

| Chữ / glyph | Trên | Light | Dark | Cần |
|---|---|---|---|---|
| ink | side | 14.72 | 14.23 | 4.5 |
| ink | surface | 16.48 | 12.28 | 4.5 |
| ink | raised | 15.79 | 11.34 | 4.5 |
| ink-muted | side | 6.98 | 8.19 | 4.5 |
| ink-muted | surface | 7.81 | 7.07 | 4.5 |
| ink-muted | raised | 7.49 | 6.53 | 4.5 |
| ink-faint | side | 5.37 | 6.31 | 4.5 |
| ink-faint | surface | 6.01 | 5.45 | 4.5 |
| ink-faint | raised | 5.76 | 5.03 | 4.5 |
| accent-ink | side | 6.54 | 8.61 | 4.5 |
| accent-ink | surface | 7.32 | 7.43 | 4.5 |
| accent-ink | raised | 7.01 | 6.87 | 4.5 |
| working | side | 5.44 | 8.08 | 4.5 |
| working | surface | 6.09 | 6.98 | 4.5 |
| working | raised | 5.84 | 6.44 | 4.5 |
| waiting | side | 5.70 | 10.28 | 4.5 |
| waiting | surface | 6.38 | 8.87 | 4.5 |
| waiting | raised | 6.12 | 8.20 | 4.5 |
| idle | side | 5.37 | 6.31 | 4.5 |
| idle | surface | 6.01 | 5.45 | 4.5 |
| idle | raised | 5.76 | 5.03 | 4.5 |
| done | side | 5.63 | 9.30 | 4.5 |
| done | surface | 6.30 | 8.03 | 4.5 |
| done | raised | 6.04 | 7.42 | 4.5 |
| error | side | 5.20 | 7.43 | 4.5 |
| error | surface | 5.82 | 6.42 | 4.5 |
| error | raised | 5.58 | 5.93 | 4.5 |
| diff-add | side | 5.63 | 9.30 | 4.5 |
| diff-add | surface | 6.30 | 8.03 | 4.5 |
| diff-add | raised | 6.04 | 7.42 | 4.5 |
| diff-del | side | 5.20 | 7.43 | 4.5 |
| diff-del | surface | 5.82 | 6.42 | 4.5 |
| diff-del | raised | 5.58 | 5.93 | 4.5 |
| working | working-soft | 5.12 | 5.54 | 4.5 |
| waiting | waiting-soft | 5.21 | 6.44 | 4.5 |
| error | error-soft | 4.80 | 5.21 | 4.5 |
| done | done-soft | 5.14 | 5.98 | 4.5 |
| diff-add | diff-add-soft | 5.14 | 5.98 | 4.5 |
| diff-del | diff-del-soft | 4.80 | 5.21 | 4.5 |
| accent-ink | accent-ink-soft | 5.96 | 5.84 | 4.5 |
| on-accent | accent | 6.06 | 6.65 | 4.5 |
| ink | control | 16.48 | 10.12 | 4.5 |
| accent | surface | 6.06 | 5.83 | 3 |
| agent-claude | surface | 4.61 | 5.33 | 3 |
| agent-codex | surface | 6.15 | 6.33 | 3 |
| agent-shell | surface | 5.31 | 7.05 | 3 |
| well-ink | well | 16.48 | 15.01 | 4.5 |
| well-dim | well | 6.01 | 5.57 | 4.5 |
| well-accent | well | 7.32 | 9.09 | 4.5 |
| well-green | well | 6.30 | 9.81 | 4.5 |
| well-red | well | 5.82 | 7.84 | 4.5 |
| well-blue | well | 6.09 | 8.89 | 4.5 |
| well-yellow | well | 6.38 | 10.85 | 4.5 |

## Xanh ngọc (`teal`): Xanh ngọc

| Token | Light | Dark |
|---|---|---|
| canvas | `#f7f7f7` | `#1e1e1e` |
| side | `#f2f2f2` | `#181818` |
| surface | `#ffffff` | `#252525` |
| raised | `#fafafa` | `#2b2b2b` |
| control | `#ffffff` | `#333333` |
| line | `#e5e5e5` | `#333333` |
| line-strong | `#d4d4d4` | `#444444` |
| ink | `#1f1f1f` | `#e6e6e6` |
| ink-muted | `#525252` | `#b0b0b0` |
| ink-faint | `#636363` | `#9a9a9a` |
| working | `#1f5fbf` | `#7fb0ff` |
| waiting | `#7a5a00` | `#e5c07b` |
| idle | `#636363` | `#9a9a9a` |
| done | `#1b6e37` | `#6fcf8f` |
| error | `#c02a2a` | `#f28b82` |
| agent-claude | `#b85a35` | `#d9825f` |
| agent-codex | `#6750b8` | `#a89cf0` |
| agent-shell | `#17786f` | `#5cc0b4` |
| diff-add | `#1b6e37` | `#6fcf8f` |
| diff-del | `#c02a2a` | `#f28b82` |
| well | `#ffffff` | `#121212` |
| well-ink | `#1f1f1f` | `#e6e6e6` |
| well-dim | `#636363` | `#8c8c8c` |
| well-green | `#1b6e37` | `#6fcf8f` |
| well-red | `#c02a2a` | `#f28b82` |
| well-blue | `#1f5fbf` | `#8ab4f8` |
| well-yellow | `#7a5a00` | `#e5c07b` |
| accent | `#0f7575` | `#4cc3c3` |
| accent-ink | `#0c6767` | `#72d6d6` |
| on-accent | `#ffffff` | `#052222` |
| well-accent | `#0c6767` | `#72d6d6` |

Contrast min chữ: light **4.80**, dark **5.03** (cần ≥ 4.5).

| Chữ / glyph | Trên | Light | Dark | Cần |
|---|---|---|---|---|
| ink | side | 14.72 | 14.23 | 4.5 |
| ink | surface | 16.48 | 12.28 | 4.5 |
| ink | raised | 15.79 | 11.34 | 4.5 |
| ink-muted | side | 6.98 | 8.19 | 4.5 |
| ink-muted | surface | 7.81 | 7.07 | 4.5 |
| ink-muted | raised | 7.49 | 6.53 | 4.5 |
| ink-faint | side | 5.37 | 6.31 | 4.5 |
| ink-faint | surface | 6.01 | 5.45 | 4.5 |
| ink-faint | raised | 5.76 | 5.03 | 4.5 |
| accent-ink | side | 5.95 | 10.40 | 4.5 |
| accent-ink | surface | 6.66 | 8.98 | 4.5 |
| accent-ink | raised | 6.38 | 8.30 | 4.5 |
| working | side | 5.44 | 8.08 | 4.5 |
| working | surface | 6.09 | 6.98 | 4.5 |
| working | raised | 5.84 | 6.44 | 4.5 |
| waiting | side | 5.70 | 10.28 | 4.5 |
| waiting | surface | 6.38 | 8.87 | 4.5 |
| waiting | raised | 6.12 | 8.20 | 4.5 |
| idle | side | 5.37 | 6.31 | 4.5 |
| idle | surface | 6.01 | 5.45 | 4.5 |
| idle | raised | 5.76 | 5.03 | 4.5 |
| done | side | 5.63 | 9.30 | 4.5 |
| done | surface | 6.30 | 8.03 | 4.5 |
| done | raised | 6.04 | 7.42 | 4.5 |
| error | side | 5.20 | 7.43 | 4.5 |
| error | surface | 5.82 | 6.42 | 4.5 |
| error | raised | 5.58 | 5.93 | 4.5 |
| diff-add | side | 5.63 | 9.30 | 4.5 |
| diff-add | surface | 6.30 | 8.03 | 4.5 |
| diff-add | raised | 6.04 | 7.42 | 4.5 |
| diff-del | side | 5.20 | 7.43 | 4.5 |
| diff-del | surface | 5.82 | 6.42 | 4.5 |
| diff-del | raised | 5.58 | 5.93 | 4.5 |
| working | working-soft | 5.12 | 5.54 | 4.5 |
| waiting | waiting-soft | 5.21 | 6.44 | 4.5 |
| error | error-soft | 4.80 | 5.21 | 4.5 |
| done | done-soft | 5.14 | 5.98 | 4.5 |
| diff-add | diff-add-soft | 5.14 | 5.98 | 4.5 |
| diff-del | diff-del-soft | 4.80 | 5.21 | 4.5 |
| accent-ink | accent-ink-soft | 5.48 | 6.89 | 4.5 |
| on-accent | accent | 5.49 | 7.87 | 4.5 |
| ink | control | 16.48 | 10.12 | 4.5 |
| accent | surface | 5.49 | 7.23 | 3 |
| agent-claude | surface | 4.61 | 5.33 | 3 |
| agent-codex | surface | 6.15 | 6.33 | 3 |
| agent-shell | surface | 5.31 | 7.05 | 3 |
| well-ink | well | 16.48 | 15.01 | 4.5 |
| well-dim | well | 6.01 | 5.57 | 4.5 |
| well-accent | well | 6.66 | 10.98 | 4.5 |
| well-green | well | 6.30 | 9.81 | 4.5 |
| well-red | well | 5.82 | 7.84 | 4.5 |
| well-blue | well | 6.09 | 8.89 | 4.5 |
| well-yellow | well | 6.38 | 10.85 | 4.5 |

## Theme hiện có: đề xuất Giữ / Bỏ

| Theme | Đề xuất | Lý do |
|---|---|---|
| Sương kính (`mist`) | Giữ | mặc định hiện tại, giữ để không đổi bất ngờ |
| Tinh vân (`nebula`) | Bỏ | tím bão hoà, glow mạnh, mỏi mắt |
| Cực quang (`aurora`) | Bỏ | gần trùng Sương kính (cùng xanh mòng két) |
| Than chì (`graphite`) | Giữ | trung tính, dịu, gần với nhóm mới |
| Bình minh sương (`dawn`) | Bỏ | gần trùng Orrery (cam ấm) |
| Ngân hà (`galaxy`) | Giữ | dark mặc định từ v96, có sao |
| Orrery (`orrery`) | Giữ | nhận diện app |
| Console (`console`) | Bỏ | cyan neon, chói |
| Tự chọn (`custom`) | Giữ | công cụ tự chỉnh, không phải preset |
| Classic: Apple, Xcode, GitHub, Solarized, Islands, Monokai, Dracula, Material, One Dark | Bỏ | classic bị bỏ ở 4.1.0; bảng màu IDE cũ, ít dùng |


## Ngẫu nhiên: thuật toán trộn màu

Chủ nhân 20/11: "Ngẫu nhiên" tự trộn bảng màu mới mỗi lần mở app, không chọn preset có sẵn. Code tham chiếu chạy được: `mauGenerate(scheme, seed)` trong `.planning/site/public/mau/palettes.js` (kèm `mauOklchToHex`, `mauRng`, `mauAllowedHues`, `mauChecks`).

**Input**: `scheme` (light / dark theo hệ thống), `seed` (số nguyên; random lúc mở app, hoặc seed đã ghim bằng "Giữ màu này"). Cùng seed + scheme luôn ra cùng bảng (RNG mulberry32), nên chỉ cần lưu seed.

**Bước**
1. Hue accent: chọn ngẫu nhiên đều trong các hue OKLCH (0 đến 359, bước 1°) cách mọi hue cần tránh ≥ 18°:
   error 25, agent-claude 45, waiting 85, done 145, agent-shell 185, working 255, agent-codex 290.
   Còn lại 130 hue: 308 đến 7 (hồng, mận), 63 đến 67, 103 đến 127 (ô liu, vàng chanh), 163 đến 167, 203 đến 237 (xanh trời, cyan).
2. Tint nền: chroma `tint` ngẫu nhiên 0 đến 0.006 theo hue đó (gần như xám, giữ "basic"); ink dùng 0.6 lần tint.
3. Thang lightness OKLCH cố định (bảng dưới), đổi sang sRGB; ngoài gamut thì giảm chroma từng 0.005 tới khi vào gamut.
4. Accent: chroma 0.08 đến 0.13 (dịu); lightness bắt đầu light 0.50 đến 0.56, dark 0.72 đến 0.80. `accent-ink` = L - 0.05 (light) / L + 0.06 (dark), chroma 0.9 lần. `on-accent` = `#ffffff` (light) / OKLCH(0.20, 0.03, hue) (dark). `well-accent` = `accent-ink`.
5. Status, diff, agent, màu chữ trên well: lấy nguyên từ `MAU_BASE` (giống các bảng preset ở trên), không trộn.
6. Kiểm `mauChecks` (đủ luật contrast như phần Nguyên tắc). Không đạt thì chỉnh accent L từng 0.015 (light tối đi, dark sáng lên), max 30 lần; vẫn không đạt thì chọn hue khác, max 20 lần; hết lượt thì dùng preset Xanh.
7. Đo trên 2000 seed x 2 scheme: 0 lần rơi về fallback, 0 bảng trượt AA.

**Thang lightness (OKLCH L)**

| Token | Light | Dark |
|---|---|---|
| canvas | 0.976 | 0.235 |
| side | 0.962 | 0.205 |
| surface | 1.000 | 0.255 |
| raised | 0.986 | 0.278 |
| control | 1.000 | 0.310 |
| line | 0.918 | 0.310 |
| line-strong | 0.860 | 0.370 |
| ink | 0.235 | 0.925 |
| ink-muted | 0.440 | 0.770 |
| ink-faint | 0.500 | 0.700 |
| well | 0.995 | 0.175 |
| well-ink | 0.235 | 0.925 |
| well-dim | 0.500 | 0.660 |

**UI trên trang mockup**: card Ngẫu nhiên có preview sống của bảng vừa trộn, nút "Trộn lại" (seed mới), Sáng / Tối riêng cho card, "Giữ màu này" (ghim seed vào danh sách, copy kèm seed + hue + accent light / dark), switch bật / tắt.

**Khi làm thật**: lưu `ccm.next.mix` = { on, seed hiện tại, pins[] }; tính bảng ở `index.html` trước khi mount (như `startTheme`) để không nháy màu; test contrast giống `next/palettes.test.ts` chạy trên vài trăm seed.

Ngẫu nhiên kiểu cũ (chọn preset): bỏ, thay bằng trộn màu mới.