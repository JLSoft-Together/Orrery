# RESEARCH v108: bảng màu dịu mắt (2026-11-20)

Mockup: `.planning/site/public/mau/index.html` (data: `mau/palettes.js`, cùng nguồn với file này). Chủ nhân chọn trên trang, bấm "Copy lựa chọn" rồi dán lại.

Mục tiêu: màu tổng thể hài hoà, ít bão hoà, đỡ mỏi mắt; thêm chế độ "Ngẫu nhiên" (mở app chọn 1 preset trong các preset giữ + mới, theo light / dark hệ thống); bỏ theme trùng.

## Nguyên tắc chung cho 5 bảng

- Nền 5 lớp gần nhau (canvas, side, surface, raised, control), lệch nhau 2 đến 4% độ sáng, cùng 1 hue nhạt của bảng. Không gradient, không glow.
- Accent saturation thấp, chỉ dùng cho nút chính, chọn, focus, tab đang mở (COLOR-SYSTEM rule 4). Accent không trùng hue với agent Claude (cam).
- Status giữ hue cố định mọi bảng (working xanh dương, waiting vàng hổ phách, idle xám, done xanh lá, error đỏ), chỉ chỉnh sáng / bão hoà theo bảng. Status luôn glyph + chữ + màu.
- Agent: Claude cam đất, Codex tím, shell xanh mòng két, chỉnh nhẹ theo bảng.
- Terminal well: tối, ám cùng hue với bảng (không đen tuyền). Riêng **Giấy light** dùng well sáng màu giấy: bỏ khối đen to giữa giao diện sáng, là nguồn chính gây chói hiện tại.
- Soft tint = `color-mix(in srgb, <màu> N%, surface)` với N: working 12, waiting 14, error 12, done 14, diff-add 14, diff-del 12, accent-soft 14.
- Contrast (WCAG): chữ ≥ 4.5:1 trên canvas / side / surface / raised và trên soft tint của chính nó; on-accent trên accent ≥ 4.5; glyph (accent, agent) ≥ 3 trên surface; chữ trên well ≥ 4.5. Tất cả 10 biến thể đều đạt.

## Map sang token app

| Mockup | Token `--ccm-*` |
|---|---|
| side | side / panel-bg |
| agent-shell | agent-shell-mark |
| well-accent | ansi prompt (brBlue / accent ink trên well) |
| well-green / red / blue / yellow | ansi green / red / blue / yellow, diff-add-well / diff-del-well |
| còn lại | trùng tên |

## Sương xám (`fog`): Lạnh, trung tính

| Token | Light | Dark |
|---|---|---|
| canvas | `#eef1f4` | `#14181c` |
| side | `#e7ebef` | `#171c21` |
| surface | `#f9fafb` | `#1c2228` |
| raised | `#f2f4f6` | `#222930` |
| control | `#ffffff` | `#2a323a` |
| line | `#dde2e7` | `#2b333b` |
| line-strong | `#c9d0d7` | `#3a444e` |
| ink | `#1f2933` | `#e3e8ec` |
| ink-muted | `#4b5763` | `#a9b4be` |
| ink-faint | `#5c6874` | `#95a1ac` |
| accent | `#3d6f8e` | `#7fa9c4` |
| accent-ink | `#335f7b` | `#9cc0d6` |
| on-accent | `#ffffff` | `#0f1c25` |
| working | `#2f5f9e` | `#8fb2e6` |
| waiting | `#7a5a10` | `#e0c27a` |
| idle | `#5c6874` | `#9ca8b3` |
| done | `#2f6b4a` | `#8cc9a5` |
| error | `#a63d40` | `#eb9fa1` |
| agent-claude | `#b8603f` | `#d4876a` |
| agent-codex | `#6656b0` | `#a89ce6` |
| agent-shell | `#2f7a72` | `#6cc0b5` |
| diff-add | `#2f6b4a` | `#8cc9a5` |
| diff-del | `#a63d40` | `#eb9fa1` |
| well | `#1e252c` | `#101418` |
| well-ink | `#dfe5ea` | `#dfe5ea` |
| well-dim | `#8f9ca8` | `#8693a0` |
| well-accent | `#9cc3d8` | `#9cc3d8` |
| well-green | `#8fcfa6` | `#8fcfa6` |
| well-red | `#eba0a2` | `#eba0a2` |
| well-blue | `#9dbde3` | `#9dbde3` |
| well-yellow | `#e2c98a` | `#e2c98a` |

Contrast min chữ: light **4.75**, dark **5.58** (cần ≥ 4.5).

| Chữ / glyph | Trên | Light | Dark | Cần |
|---|---|---|---|---|
| ink | side | 12.32 | 13.90 | 4.5 |
| ink | surface | 14.12 | 13.00 | 4.5 |
| ink | raised | 13.38 | 11.92 | 4.5 |
| ink-muted | side | 6.17 | 8.13 | 4.5 |
| ink-muted | surface | 7.07 | 7.61 | 4.5 |
| ink-muted | raised | 6.70 | 6.97 | 4.5 |
| ink-faint | side | 4.75 | 6.51 | 4.5 |
| ink-faint | surface | 5.45 | 6.09 | 4.5 |
| ink-faint | raised | 5.17 | 5.58 | 4.5 |
| accent-ink | side | 5.72 | 8.92 | 4.5 |
| accent-ink | surface | 6.56 | 8.35 | 4.5 |
| accent-ink | raised | 6.22 | 7.65 | 4.5 |
| working | side | 5.39 | 7.90 | 4.5 |
| working | surface | 6.18 | 7.39 | 4.5 |
| working | raised | 5.86 | 6.78 | 4.5 |
| waiting | side | 5.32 | 9.94 | 4.5 |
| waiting | surface | 6.09 | 9.30 | 4.5 |
| waiting | raised | 5.78 | 8.52 | 4.5 |
| idle | side | 4.75 | 7.08 | 4.5 |
| idle | surface | 5.45 | 6.62 | 4.5 |
| idle | raised | 5.17 | 6.07 | 4.5 |
| done | side | 5.27 | 8.99 | 4.5 |
| done | surface | 6.05 | 8.41 | 4.5 |
| done | raised | 5.73 | 7.71 | 4.5 |
| error | side | 5.21 | 8.17 | 4.5 |
| error | surface | 5.98 | 7.65 | 4.5 |
| error | raised | 5.66 | 7.01 | 4.5 |
| diff-add | side | 5.27 | 8.99 | 4.5 |
| diff-add | surface | 6.05 | 8.41 | 4.5 |
| diff-add | raised | 5.73 | 7.71 | 4.5 |
| diff-del | side | 5.21 | 8.17 | 4.5 |
| diff-del | surface | 5.98 | 7.65 | 4.5 |
| diff-del | raised | 5.66 | 7.01 | 4.5 |
| working | working-soft | 5.19 | 5.89 | 4.5 |
| waiting | waiting-soft | 5.01 | 6.85 | 4.5 |
| error | error-soft | 4.99 | 6.08 | 4.5 |
| done | done-soft | 4.96 | 6.28 | 4.5 |
| diff-add | diff-add-soft | 4.96 | 6.28 | 4.5 |
| diff-del | diff-del-soft | 4.99 | 6.08 | 4.5 |
| accent-ink | accent-ink-soft | 5.48 | 6.51 | 4.5 |
| on-accent | accent | 5.44 | 6.90 | 4.5 |
| ink | control | 14.76 | 10.53 | 4.5 |
| accent | surface | 5.20 | 6.40 | 3 |
| agent-claude | surface | 4.20 | 5.71 | 3 |
| agent-codex | surface | 5.70 | 6.54 | 3 |
| agent-shell | surface | 4.84 | 7.52 | 3 |
| well-ink | well | 12.19 | 14.56 | 4.5 |
| well-dim | well | 5.52 | 5.90 | 4.5 |
| well-accent | well | 8.27 | 9.88 | 4.5 |
| well-green | well | 8.59 | 10.26 | 4.5 |
| well-red | well | 7.43 | 8.88 | 4.5 |
| well-blue | well | 7.98 | 9.53 | 4.5 |
| well-yellow | well | 9.55 | 11.41 | 4.5 |

## Rêu (`moss`): Xanh lá dịu

| Token | Light | Dark |
|---|---|---|
| canvas | `#eff1ec` | `#151915` |
| side | `#e8ebe4` | `#181c17` |
| surface | `#fafbf8` | `#1d221c` |
| raised | `#f3f5f0` | `#232922` |
| control | `#ffffff` | `#2b322a` |
| line | `#dde1d7` | `#2c332b` |
| line-strong | `#c8cec0` | `#3b4439` |
| ink | `#20271f` | `#e4e8df` |
| ink-muted | `#4d574b` | `#aab3a5` |
| ink-faint | `#5c6759` | `#97a191` |
| accent | `#4b7556` | `#94b89c` |
| accent-ink | `#3f6649` | `#a8c8ae` |
| on-accent | `#ffffff` | `#13211a` |
| working | `#3d6491` | `#93b3dc` |
| waiting | `#7a5c12` | `#dcc27f` |
| idle | `#5c6759` | `#9ea799` |
| done | `#2e6b3c` | `#9fd0a6` |
| error | `#a8433c` | `#e9a098` |
| agent-claude | `#b5603e` | `#d38a6c` |
| agent-codex | `#6a5aa8` | `#ab9fe0` |
| agent-shell | `#2f766c` | `#79c2b4` |
| diff-add | `#2e6b3c` | `#9fd0a6` |
| diff-del | `#a8433c` | `#e9a098` |
| well | `#222821` | `#101310` |
| well-ink | `#e1e6dc` | `#e1e6dc` |
| well-dim | `#929d8f` | `#87927f` |
| well-accent | `#a8c8ae` | `#a8c8ae` |
| well-green | `#9fd0a6` | `#9fd0a6` |
| well-red | `#eaa199` | `#eaa199` |
| well-blue | `#a2bfe0` | `#a2bfe0` |
| well-yellow | `#e0c98a` | `#e0c98a` |

Contrast min chữ: light **4.81**, dark **5.54** (cần ≥ 4.5).

| Chữ / glyph | Trên | Light | Dark | Cần |
|---|---|---|---|---|
| ink | side | 12.71 | 13.89 | 4.5 |
| ink | surface | 14.74 | 13.03 | 4.5 |
| ink | raised | 13.95 | 11.98 | 4.5 |
| ink-muted | side | 6.27 | 7.97 | 4.5 |
| ink-muted | surface | 7.27 | 7.47 | 4.5 |
| ink-muted | raised | 6.88 | 6.87 | 4.5 |
| ink-faint | side | 4.93 | 6.43 | 4.5 |
| ink-faint | surface | 5.71 | 6.03 | 4.5 |
| ink-faint | raised | 5.40 | 5.54 | 4.5 |
| accent-ink | side | 5.43 | 9.48 | 4.5 |
| accent-ink | surface | 6.30 | 8.89 | 4.5 |
| accent-ink | raised | 5.96 | 8.17 | 4.5 |
| working | side | 5.08 | 7.99 | 4.5 |
| working | surface | 5.89 | 7.49 | 4.5 |
| working | raised | 5.58 | 6.89 | 4.5 |
| waiting | side | 5.18 | 9.91 | 4.5 |
| waiting | surface | 6.00 | 9.30 | 4.5 |
| waiting | raised | 5.68 | 8.55 | 4.5 |
| idle | side | 4.93 | 6.93 | 4.5 |
| idle | surface | 5.71 | 6.50 | 4.5 |
| idle | raised | 5.40 | 5.98 | 4.5 |
| done | side | 5.31 | 9.90 | 4.5 |
| done | surface | 6.16 | 9.28 | 4.5 |
| done | raised | 5.82 | 8.53 | 4.5 |
| error | side | 4.93 | 8.17 | 4.5 |
| error | surface | 5.72 | 7.66 | 4.5 |
| error | raised | 5.41 | 7.05 | 4.5 |
| diff-add | side | 5.31 | 9.90 | 4.5 |
| diff-add | surface | 6.16 | 9.28 | 4.5 |
| diff-add | raised | 5.82 | 8.53 | 4.5 |
| diff-del | side | 4.93 | 8.17 | 4.5 |
| diff-del | surface | 5.72 | 7.66 | 4.5 |
| diff-del | raised | 5.41 | 7.05 | 4.5 |
| working | working-soft | 4.99 | 5.99 | 4.5 |
| waiting | waiting-soft | 4.94 | 6.83 | 4.5 |
| error | error-soft | 4.81 | 6.12 | 4.5 |
| done | done-soft | 5.04 | 6.82 | 4.5 |
| diff-add | diff-add-soft | 5.04 | 6.82 | 4.5 |
| diff-del | diff-del-soft | 4.81 | 6.12 | 4.5 |
| accent-ink | accent-ink-soft | 5.26 | 6.78 | 4.5 |
| on-accent | accent | 5.28 | 7.61 | 4.5 |
| ink | control | 15.31 | 10.62 | 4.5 |
| accent | surface | 5.08 | 7.39 | 3 |
| agent-claude | surface | 4.30 | 5.87 | 3 |
| agent-codex | surface | 5.55 | 6.76 | 3 |
| agent-shell | surface | 5.15 | 7.85 | 3 |
| well-ink | well | 11.88 | 14.75 | 4.5 |
| well-dim | well | 5.34 | 5.74 | 4.5 |
| well-accent | well | 8.28 | 10.28 | 4.5 |
| well-green | well | 8.65 | 10.73 | 4.5 |
| well-red | well | 7.22 | 8.96 | 4.5 |
| well-blue | well | 7.94 | 9.86 | 4.5 |
| well-yellow | well | 9.25 | 11.48 | 4.5 |

## Giấy (`paper`): Ấm, terminal sáng

| Token | Light | Dark |
|---|---|---|
| canvas | `#f3efe8` | `#1a1715` |
| side | `#ede8e0` | `#1d1a17` |
| surface | `#fbf9f5` | `#221f1c` |
| raised | `#f5f1eb` | `#292522` |
| control | `#fffdfa` | `#312c28` |
| line | `#e2dcd2` | `#332e2a` |
| line-strong | `#cfc7ba` | `#443d37` |
| ink | `#2a2420` | `#ece6df` |
| ink-muted | `#5a5048` | `#b5aca2` |
| ink-faint | `#685e55` | `#a0978d` |
| accent | `#4a6585` | `#9db4cf` |
| accent-ink | `#405872` | `#b0c4db` |
| on-accent | `#ffffff` | `#16202c` |
| working | `#3b5f8f` | `#9db4dc` |
| waiting | `#7a580c` | `#dfc282` |
| idle | `#685e55` | `#a39a91` |
| done | `#3d6a3a` | `#a3c99a` |
| error | `#a53b35` | `#e89d95` |
| agent-claude | `#b0573a` | `#d98c6c` |
| agent-codex | `#6a58a8` | `#b1a3e0` |
| agent-shell | `#2e7268` | `#7fc0b0` |
| diff-add | `#3d6a3a` | `#a3c99a` |
| diff-del | `#a53b35` | `#e89d95` |
| well | `#f0ebe3` | `#141210` |
| well-ink | `#2a2420` | `#e6e0d8` |
| well-dim | `#665c53` | `#8d8379` |
| well-accent | `#405872` | `#b0c4db` |
| well-green | `#38623a` | `#a3c99a` |
| well-red | `#9c3832` | `#e89d95` |
| well-blue | `#3b5b88` | `#a5bde0` |
| well-yellow | `#6f520c` | `#dfc282` |

Contrast min chữ: light **4.94**, dark **5.03** (cần ≥ 4.5).

| Chữ / glyph | Trên | Light | Dark | Cần |
|---|---|---|---|---|
| ink | side | 12.55 | 13.98 | 4.5 |
| ink | surface | 14.56 | 13.23 | 4.5 |
| ink | raised | 13.61 | 12.27 | 4.5 |
| ink-muted | side | 6.44 | 7.74 | 4.5 |
| ink-muted | surface | 7.46 | 7.33 | 4.5 |
| ink-muted | raised | 6.98 | 6.79 | 4.5 |
| ink-faint | side | 5.19 | 6.03 | 4.5 |
| ink-faint | surface | 6.01 | 5.70 | 4.5 |
| ink-faint | raised | 5.62 | 5.29 | 4.5 |
| accent-ink | side | 6.03 | 9.70 | 4.5 |
| accent-ink | surface | 6.99 | 9.19 | 4.5 |
| accent-ink | raised | 6.53 | 8.51 | 4.5 |
| working | side | 5.35 | 8.24 | 4.5 |
| working | surface | 6.20 | 7.80 | 4.5 |
| working | raised | 5.80 | 7.23 | 4.5 |
| waiting | side | 5.33 | 10.04 | 4.5 |
| waiting | surface | 6.19 | 9.51 | 4.5 |
| waiting | raised | 5.78 | 8.81 | 4.5 |
| idle | side | 5.19 | 6.26 | 4.5 |
| idle | surface | 6.01 | 5.92 | 4.5 |
| idle | raised | 5.62 | 5.49 | 4.5 |
| done | side | 5.18 | 9.39 | 4.5 |
| done | surface | 6.01 | 8.88 | 4.5 |
| done | raised | 5.62 | 8.23 | 4.5 |
| error | side | 5.26 | 7.99 | 4.5 |
| error | surface | 6.09 | 7.56 | 4.5 |
| error | raised | 5.70 | 7.01 | 4.5 |
| diff-add | side | 5.18 | 9.39 | 4.5 |
| diff-add | surface | 6.01 | 8.88 | 4.5 |
| diff-add | raised | 5.62 | 8.23 | 4.5 |
| diff-del | side | 5.26 | 7.99 | 4.5 |
| diff-del | surface | 6.09 | 7.56 | 4.5 |
| diff-del | raised | 5.70 | 7.01 | 4.5 |
| working | working-soft | 5.25 | 6.18 | 4.5 |
| waiting | waiting-soft | 5.05 | 6.95 | 4.5 |
| error | error-soft | 5.09 | 6.03 | 4.5 |
| done | done-soft | 4.94 | 6.57 | 4.5 |
| diff-add | diff-add-soft | 4.94 | 6.57 | 4.5 |
| diff-del | diff-del-soft | 5.09 | 6.03 | 4.5 |
| accent-ink | accent-ink-soft | 5.76 | 6.99 | 4.5 |
| on-accent | accent | 6.01 | 7.72 | 4.5 |
| ink | control | 15.08 | 11.14 | 4.5 |
| accent | surface | 5.72 | 7.70 | 3 |
| agent-claude | surface | 4.68 | 6.18 | 3 |
| agent-codex | surface | 5.59 | 7.17 | 3 |
| agent-shell | surface | 5.36 | 7.86 | 3 |
| well-ink | well | 12.90 | 14.25 | 4.5 |
| well-dim | well | 5.49 | 5.03 | 4.5 |
| well-accent | well | 6.19 | 10.47 | 4.5 |
| well-green | well | 5.95 | 10.13 | 4.5 |
| well-red | well | 5.85 | 8.62 | 4.5 |
| well-blue | well | 5.83 | 9.75 | 4.5 |
| well-yellow | well | 6.13 | 10.84 | 4.5 |

## Chàm (`indigo`): Tím than nhạt

| Token | Light | Dark |
|---|---|---|
| canvas | `#f0f0f5` | `#15151d` |
| side | `#e9e9f0` | `#181821` |
| surface | `#fafafd` | `#1d1d27` |
| raised | `#f3f3f8` | `#23232f` |
| control | `#ffffff` | `#2b2b38` |
| line | `#dfdfe8` | `#2c2c39` |
| line-strong | `#cacad7` | `#3b3b4b` |
| ink | `#232336` | `#e6e6f0` |
| ink-muted | `#4f4f66` | `#ababc2` |
| ink-faint | `#5f5f77` | `#9898b0` |
| accent | `#585ca4` | `#a3a7e6` |
| accent-ink | `#4c5096` | `#b6b9ee` |
| on-accent | `#ffffff` | `#171a3a` |
| working | `#2a6585` | `#8cc4dc` |
| waiting | `#7a5a14` | `#e2c784` |
| idle | `#5f5f77` | `#a0a0b6` |
| done | `#2f6b51` | `#92ceb0` |
| error | `#a5405a` | `#eba0b2` |
| agent-claude | `#b65f42` | `#d98b6e` |
| agent-codex | `#7656b0` | `#bea3ec` |
| agent-shell | `#2b7774` | `#78c4bd` |
| diff-add | `#2f6b51` | `#92ceb0` |
| diff-del | `#a5405a` | `#eba0b2` |
| well | `#1d1d2b` | `#101017` |
| well-ink | `#e2e2ee` | `#e2e2ee` |
| well-dim | `#9292ab` | `#8a8aa3` |
| well-accent | `#b6b9ee` | `#b6b9ee` |
| well-green | `#92ceb0` | `#92ceb0` |
| well-red | `#eba3b4` | `#eba3b4` |
| well-blue | `#9ccbe0` | `#9ccbe0` |
| well-yellow | `#e2c784` | `#e2c784` |

Contrast min chữ: light **4.88**, dark **5.51** (cần ≥ 4.5).

| Chữ / glyph | Trên | Light | Dark | Cần |
|---|---|---|---|---|
| ink | side | 12.73 | 14.22 | 4.5 |
| ink | surface | 14.77 | 13.47 | 4.5 |
| ink | raised | 13.91 | 12.52 | 4.5 |
| ink-muted | side | 6.58 | 7.84 | 4.5 |
| ink-muted | surface | 7.63 | 7.43 | 4.5 |
| ink-muted | raised | 7.18 | 6.90 | 4.5 |
| ink-faint | side | 5.13 | 6.26 | 4.5 |
| ink-faint | surface | 5.95 | 5.93 | 4.5 |
| ink-faint | raised | 5.60 | 5.51 | 4.5 |
| accent-ink | side | 6.00 | 9.37 | 4.5 |
| accent-ink | surface | 6.96 | 8.88 | 4.5 |
| accent-ink | raised | 6.56 | 8.25 | 4.5 |
| working | side | 5.27 | 9.27 | 4.5 |
| working | surface | 6.11 | 8.78 | 4.5 |
| working | raised | 5.76 | 8.16 | 4.5 |
| waiting | side | 5.27 | 10.69 | 4.5 |
| waiting | surface | 6.11 | 10.13 | 4.5 |
| waiting | raised | 5.75 | 9.41 | 4.5 |
| idle | side | 5.13 | 6.88 | 4.5 |
| idle | surface | 5.95 | 6.52 | 4.5 |
| idle | raised | 5.60 | 6.06 | 4.5 |
| done | side | 5.20 | 9.80 | 4.5 |
| done | surface | 6.03 | 9.29 | 4.5 |
| done | raised | 5.68 | 8.63 | 4.5 |
| error | side | 4.99 | 8.56 | 4.5 |
| error | surface | 5.79 | 8.12 | 4.5 |
| error | raised | 5.45 | 7.54 | 4.5 |
| diff-add | side | 5.20 | 9.80 | 4.5 |
| diff-add | surface | 6.03 | 9.29 | 4.5 |
| diff-add | raised | 5.68 | 8.63 | 4.5 |
| diff-del | side | 4.99 | 8.56 | 4.5 |
| diff-del | surface | 5.79 | 8.12 | 4.5 |
| diff-del | raised | 5.45 | 7.54 | 4.5 |
| working | working-soft | 5.15 | 6.88 | 4.5 |
| waiting | waiting-soft | 5.02 | 7.35 | 4.5 |
| error | error-soft | 4.88 | 6.42 | 4.5 |
| done | done-soft | 4.95 | 6.87 | 4.5 |
| diff-add | diff-add-soft | 4.95 | 6.87 | 4.5 |
| diff-del | diff-del-soft | 4.88 | 6.42 | 4.5 |
| accent-ink | accent-ink-soft | 5.75 | 6.86 | 4.5 |
| on-accent | accent | 6.03 | 7.41 | 4.5 |
| ink | control | 15.38 | 11.25 | 4.5 |
| accent | surface | 5.79 | 7.34 | 3 |
| agent-claude | surface | 4.28 | 6.26 | 3 |
| agent-codex | surface | 5.41 | 7.67 | 3 |
| agent-shell | surface | 5.05 | 8.30 | 3 |
| well-ink | well | 12.94 | 14.75 | 4.5 |
| well-dim | well | 5.48 | 5.63 | 4.5 |
| well-accent | well | 8.84 | 10.07 | 4.5 |
| well-green | well | 9.25 | 10.53 | 4.5 |
| well-red | well | 8.26 | 9.41 | 4.5 |
| well-blue | well | 9.53 | 10.85 | 4.5 |
| well-yellow | well | 10.09 | 11.49 | 4.5 |

## Sen (`lotus`): Hồng tro

| Token | Light | Dark |
|---|---|---|
| canvas | `#f2f0f1` | `#181517` |
| side | `#ebe8ea` | `#1b181a` |
| surface | `#fbfafa` | `#211d20` |
| raised | `#f4f2f3` | `#282326` |
| control | `#ffffff` | `#302a2e` |
| line | `#e1dcdf` | `#322c30` |
| line-strong | `#cdc6ca` | `#433b40` |
| ink | `#2a2328` | `#ede6ea` |
| ink-muted | `#574d54` | `#b6abb2` |
| ink-faint | `#665c63` | `#a1969d` |
| accent | `#864d69` | `#d6a3bd` |
| accent-ink | `#78435d` | `#e2b6cc` |
| on-accent | `#ffffff` | `#2a1420` |
| working | `#3b6090` | `#98b6e0` |
| waiting | `#7a5a12` | `#e0c482` |
| idle | `#665c63` | `#a59aa1` |
| done | `#356a46` | `#98cca8` |
| error | `#a53e3e` | `#ec9f9b` |
| agent-claude | `#b35e3f` | `#d98b6d` |
| agent-codex | `#6457ad` | `#ada2e6` |
| agent-shell | `#2e7670` | `#7bc1b6` |
| diff-add | `#356a46` | `#98cca8` |
| diff-del | `#a53e3e` | `#ec9f9b` |
| well | `#221d21` | `#121012` |
| well-ink | `#ebe4e8` | `#ebe4e8` |
| well-dim | `#9d9198` | `#928790` |
| well-accent | `#e2b6cc` | `#e2b6cc` |
| well-green | `#98cca8` | `#98cca8` |
| well-red | `#ec9f9b` | `#ec9f9b` |
| well-blue | `#a0bce2` | `#a0bce2` |
| well-yellow | `#e0c482` | `#e0c482` |

Contrast min chữ: light **5.01**, dark **5.42** (cần ≥ 4.5).

| Chữ / glyph | Trên | Light | Dark | Cần |
|---|---|---|---|---|
| ink | side | 12.61 | 14.35 | 4.5 |
| ink | surface | 14.72 | 13.56 | 4.5 |
| ink | raised | 13.76 | 12.59 | 4.5 |
| ink-muted | side | 6.65 | 7.93 | 4.5 |
| ink-muted | surface | 7.77 | 7.50 | 4.5 |
| ink-muted | raised | 7.26 | 6.96 | 4.5 |
| ink-faint | side | 5.27 | 6.18 | 4.5 |
| ink-faint | surface | 6.15 | 5.84 | 4.5 |
| ink-faint | raised | 5.75 | 5.42 | 4.5 |
| accent-ink | side | 6.26 | 9.90 | 4.5 |
| accent-ink | surface | 7.30 | 9.35 | 4.5 |
| accent-ink | raised | 6.83 | 8.68 | 4.5 |
| working | side | 5.29 | 8.47 | 4.5 |
| working | surface | 6.18 | 8.01 | 4.5 |
| working | raised | 5.78 | 7.44 | 4.5 |
| waiting | side | 5.23 | 10.39 | 4.5 |
| waiting | surface | 6.11 | 9.82 | 4.5 |
| waiting | raised | 5.71 | 9.12 | 4.5 |
| idle | side | 5.27 | 6.49 | 4.5 |
| idle | surface | 6.15 | 6.13 | 4.5 |
| idle | raised | 5.75 | 5.69 | 4.5 |
| done | side | 5.23 | 9.68 | 4.5 |
| done | surface | 6.11 | 9.15 | 4.5 |
| done | raised | 5.71 | 8.49 | 4.5 |
| error | side | 5.14 | 8.39 | 4.5 |
| error | surface | 6.00 | 7.93 | 4.5 |
| error | raised | 5.61 | 7.36 | 4.5 |
| diff-add | side | 5.23 | 9.68 | 4.5 |
| diff-add | surface | 6.11 | 9.15 | 4.5 |
| diff-add | raised | 5.71 | 8.49 | 4.5 |
| diff-del | side | 5.14 | 8.39 | 4.5 |
| diff-del | surface | 6.00 | 7.93 | 4.5 |
| diff-del | raised | 5.61 | 7.36 | 4.5 |
| working | working-soft | 5.23 | 6.38 | 4.5 |
| waiting | waiting-soft | 5.02 | 7.19 | 4.5 |
| error | error-soft | 5.01 | 6.29 | 4.5 |
| done | done-soft | 5.01 | 6.74 | 4.5 |
| diff-add | diff-add-soft | 5.01 | 6.74 | 4.5 |
| diff-del | diff-del-soft | 5.01 | 6.29 | 4.5 |
| accent-ink | accent-ink-soft | 6.00 | 7.12 | 4.5 |
| on-accent | accent | 6.40 | 8.06 | 4.5 |
| ink | control | 15.33 | 11.43 | 4.5 |
| accent | surface | 6.15 | 7.80 | 3 |
| agent-claude | surface | 4.39 | 6.24 | 3 |
| agent-codex | surface | 5.74 | 7.21 | 3 |
| agent-shell | surface | 5.11 | 8.04 | 3 |
| well-ink | well | 13.27 | 15.15 | 4.5 |
| well-dim | well | 5.48 | 5.49 | 4.5 |
| well-accent | well | 9.32 | 10.64 | 4.5 |
| well-green | well | 9.12 | 10.41 | 4.5 |
| well-red | well | 7.90 | 9.02 | 4.5 |
| well-blue | well | 8.52 | 9.73 | 4.5 |
| well-yellow | well | 9.79 | 11.17 | 4.5 |

## Theme hiện có: đề xuất Giữ / Bỏ

| Theme | Đề xuất | Lý do |
|---|---|---|
| Sương kính (`mist`) | Giữ | mặc định hiện tại, giữ để không đổi bất ngờ |
| Tinh vân (`nebula`) | Bỏ | tím bão hoà, glow mạnh, mỏi mắt |
| Cực quang (`aurora`) | Bỏ | gần trùng Sương kính (cùng xanh mòng két) |
| Than chì (`graphite`) | Giữ | trung tính, dịu, gần với nhóm mới |
| Bình minh sương (`dawn`) | Bỏ | gần trùng Orrery + Giấy (cam ấm) |
| Ngân hà (`galaxy`) | Giữ | dark mặc định từ v96, có sao |
| Orrery (`orrery`) | Giữ | nhận diện app |
| Console (`console`) | Bỏ | cyan neon, chói |
| Tự chọn (`custom`) | Giữ | công cụ tự chỉnh, không phải preset |
| Classic: Apple, Xcode, GitHub, Solarized, Islands, Monokai, Dracula, Material, One Dark | Bỏ | classic bị bỏ ở 4.1.0; bảng màu IDE cũ, ít dùng |

Ngẫu nhiên: không lấy "Tự chọn"; ở light bỏ qua preset chỉ có dark (Ngân hà, Console) và ngược lại.
