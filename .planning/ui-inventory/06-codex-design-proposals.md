# 06 - Đề xuất design system Sương kính

Phạm vi: thông số đủ cụ thể để vẽ mockup Workspace W2 và các màn chính. Đây là proposal, chưa sửa source. Quy ước token dùng tên `--ccm-*` để map vào Orrery; màu và kính lấy hướng từ `jin-tools/apps/web/src/styles/tokens.css`.

## 1. Nguyên tắc

- Canvas là nền sương radial gradient tĩnh; chrome dùng glass blur, terminal và vùng code giữ nền đặc, tương phản cao.
- Workspace W2 là màn chính: rail sống và hàng đợi “Cần bạn” ở trái, tab/pane ở giữa, Explorer mở dạng drawer ở phải.
- Mỗi trạng thái có glyph + nhãn + màu; không dùng màu đơn độc để truyền nghĩa.
- Body text và mọi trạng thái hiển thị dạng chữ đạt WCAG AA 4.5:1 trên surface đã composite. Icon và đường biểu đồ tối thiểu 3:1.
- Contrast dưới đây là ước lượng theo sRGB compositing, làm tròn 0.1. Trước khi implement cần đo lại bằng giá trị render thực tế trên ba điểm nền gradient.

## 2. Màu

### 2.1 Sương kính sáng

| Vai trò | Token đề xuất | Giá trị |
|---|---|---|
| Canvas | `--ccm-canvas` | `#EEF3F6` |
| Nền sương, góc trên trái | `--ccm-bg-a` | radial `#C9ECE2` 0%, transparent 72%, tâm 6% 0%, size 55% 45% |
| Nền sương, góc trên phải | `--ccm-bg-b` | radial `#D3E0FB` 0%, transparent 70%, tâm 100% 6%, size 50% 55% |
| Nền sương, đáy | `--ccm-bg-c` | radial `#F6DFD4` 0%, transparent 72%, tâm 55% 104%, size 60% 50% |
| Surface thấp | `--ccm-surface` | `rgb(255 255 255 / 0.58)` |
| Surface raised | `--ccm-raised` | `rgb(255 255 255 / 0.60)` |
| Control | `--ccm-control` | `rgb(255 255 255 / 0.72)` |
| Side chrome | `--ccm-side` | `rgb(255 255 255 / 0.36)` |
| Glass popover | `--ccm-glass` | `rgb(255 255 255 / 0.74)` |
| Glass popover raised | `--ccm-glass-pop` | `rgb(250 252 253 / 0.86)` |
| Solid fallback | `--ccm-material` | `#F7FAFB` |
| Ink | `--ccm-ink` | `#1B2A33` |
| Ink muted | `--ccm-ink-muted` | `#53656F` |
| Ink faint | `--ccm-ink-faint` | `#61737B` |
| Ink disabled | `--ccm-ink-disabled` | `rgb(27 42 51 / 0.30)`, chỉ control disabled, không dùng nội dung cần đọc |
| Line | `--ccm-line` | `rgb(28 60 80 / 0.10)` |
| Line strong | `--ccm-line-strong` | `rgb(28 60 80 / 0.18)` |
| Line accent | `--ccm-line-accent` | `rgb(42 127 158 / 0.35)` |
| Glass edge | `--ccm-glass-edge` | `rgb(255 255 255 / 0.80)` |
| Accent fill | `--ccm-accent` | `#2A7F9E` |
| Accent text | `--ccm-accent-ink` | `#1D6886` |
| On accent | `--ccm-on-accent` | `#FFFFFF` |
| Accent gradient | `--ccm-accent-grad` | `linear-gradient(180deg, #3592B3, #2A7F9E)` |
| Accent soft | `--ccm-accent-soft` | `rgb(42 127 158 / 0.14)` |
| Selection | `--ccm-selection` | `rgb(42 127 158 / 0.22)` |
| Focus | `--ccm-focus` | `#2A7F9E` |
| Working | `--ccm-working` | `#9A5C08` |
| Waiting | `--ccm-waiting` | `#8A6400` |
| Idle | `--ccm-idle` | `#687A83` |
| Error | `--ccm-error` | `#BF3647` |
| Ended | `--ccm-ended` | `#687A83` |
| Done / success | `--ccm-done` | `#1D7752` |
| Soft working | `--ccm-working-soft` | `rgb(240 160 40 / 0.18)` |
| Soft waiting | `--ccm-waiting-soft` | `rgb(240 160 40 / 0.18)` |
| Soft error | `--ccm-error-soft` | `rgb(220 70 90 / 0.13)` |
| Soft done | `--ccm-done-soft` | `rgb(46 170 120 / 0.16)` |
| Claude | `--ccm-agent-claude` | `#C15F3C`, glyph/badge only |
| Codex | `--ccm-agent-codex` | `#5E4FC0`, glyph/badge only |
| Shell | `--ccm-agent-shell` | `#53656F`, glyph/badge only |
| Diff add | `--ccm-diff-add` | `#1D7752` |
| Diff delete | `--ccm-diff-del` | `#BF3647` |
| Diff modified | `--ccm-diff-mod` | `#0060A8` |
| Terminal well | `--ccm-well` | `#111A20` |
| Terminal ink | `--ccm-well-ink` | `#E4EEF2` |
| Terminal selection | `--ccm-well-selection` | `#28566A` |

### 2.2 Sương kính tối

| Vai trò | Token đề xuất | Giá trị |
|---|---|---|
| Canvas | `--ccm-canvas` | `#0F171D` |
| Nền sương, góc trên trái | `--ccm-bg-a` | radial `rgb(40 125 120 / 0.38)` tới transparent 72%, tâm 0% 0%, size 55% 45% |
| Nền sương, góc trên phải | `--ccm-bg-b` | radial `rgb(70 82 170 / 0.34)` tới transparent 70%, tâm 100% 0%, size 50% 55% |
| Nền sương, đáy | `--ccm-bg-c` | radial `rgb(150 80 95 / 0.24)` tới transparent 72%, tâm 60% 104%, size 60% 50% |
| Surface thấp | `--ccm-surface` | `rgb(255 255 255 / 0.06)` |
| Surface raised | `--ccm-raised` | `rgb(255 255 255 / 0.05)` |
| Control | `--ccm-control` | `rgb(255 255 255 / 0.07)` |
| Side chrome | `--ccm-side` | `rgb(255 255 255 / 0.035)` |
| Glass popover | `--ccm-glass` | `rgb(20 31 38 / 0.78)` |
| Glass popover raised | `--ccm-glass-pop` | `rgb(26 38 46 / 0.88)` |
| Solid fallback | `--ccm-material` | `#18232B` |
| Ink | `--ccm-ink` | `#E4EEF2` |
| Ink muted | `--ccm-ink-muted` | `#9FB2BB` |
| Ink faint | `--ccm-ink-faint` | `#7F939C` |
| Ink disabled | `--ccm-ink-disabled` | `rgb(228 238 242 / 0.28)`, chỉ control disabled |
| Line | `--ccm-line` | `rgb(255 255 255 / 0.09)` |
| Line strong | `--ccm-line-strong` | `rgb(255 255 255 / 0.16)` |
| Line accent | `--ccm-line-accent` | `rgb(115 198 220 / 0.35)` |
| Glass edge | `--ccm-glass-edge` | `rgb(255 255 255 / 0.10)` |
| Accent fill | `--ccm-accent` | `#73C6DC` |
| Accent text | `--ccm-accent-ink` | `#8FD3E5` |
| On accent | `--ccm-on-accent` | `#08202A` |
| Accent gradient | `--ccm-accent-grad` | `linear-gradient(180deg, #89D4E6, #63B7CD)` |
| Accent soft | `--ccm-accent-soft` | `rgb(115 198 220 / 0.16)` |
| Selection | `--ccm-selection` | `rgb(115 198 220 / 0.28)` |
| Focus | `--ccm-focus` | `#8FD3E5` |
| Working | `--ccm-working` | `#F2C06B` |
| Waiting | `--ccm-waiting` | `#F2D36B` |
| Idle | `--ccm-idle` | `#9FB2BB` |
| Error | `--ccm-error` | `#FF919C` |
| Ended | `--ccm-ended` | `#7F939C` |
| Done / success | `--ccm-done` | `#6FD6A6` |
| Soft working | `--ccm-working-soft` | `rgb(242 192 107 / 0.16)` |
| Soft waiting | `--ccm-waiting-soft` | `rgb(242 211 107 / 0.16)` |
| Soft error | `--ccm-error-soft` | `rgb(255 145 156 / 0.14)` |
| Soft done | `--ccm-done-soft` | `rgb(111 214 166 / 0.14)` |
| Claude | `--ccm-agent-claude` | `#D97757`, glyph/badge only |
| Codex | `--ccm-agent-codex` | `#A49CF0`, glyph/badge only |
| Shell | `--ccm-agent-shell` | `#9FB2BB`, glyph/badge only |
| Diff add | `--ccm-diff-add` | `#6FD6A6` |
| Diff delete | `--ccm-diff-del` | `#FF919C` |
| Diff modified | `--ccm-diff-mod` | `#8AB4FF` |
| Terminal well | `--ccm-well` | `#080D11` |
| Terminal ink | `--ccm-well-ink` | `#E4EEF2` |
| Terminal selection | `--ccm-well-selection` | `#28566A` |

### 2.3 Contrast on composited glass

Ratios are estimates from sRGB compositing over the average canvas, rounded to 0.1. For dark glass, use the dark ink ladder; for light glass, use the light ladder. Verify at gradient hot spots before release. Faint is reserved for nonessential metadata, but still targets AA.

| Foreground | Surface tested | Light estimate | Dark estimate | Use |
|---|---|---:|---:|---|
| Ink | Surface composite | 14.1:1 | 14.4:1 | Body, pane titles |
| Ink muted | Surface composite | 5.8:1 | 7.8:1 | Secondary labels |
| Ink faint | Surface composite | 4.7:1 | 5.3:1 | Metadata, placeholder |
| Accent ink | Surface composite | 5.9:1 | 10.2:1 | Links, selected text |
| Working | Surface composite | 5.1:1 | 10.2:1 | Status text/icon |
| Waiting | Surface composite | 5.1:1 | 11.6:1 | Status text/icon |
| Error | Surface composite | 5.2:1 | 7.9:1 | Status text/icon |
| Done | Surface composite | 5.3:1 | 9.6:1 | Status text/icon |
| On accent | Solid accent / dark accent | 4.6:1 | 9.6:1 | Filled primary control |
| Terminal ink | Well | 14:1 | 14:1 | Terminal output |

Do not reduce text opacity. If a derived palette fails the stated ratio, darken/lighten the ink or make that surface more opaque; do not accept a contrast exception. Accent colors for agent identity are glyph/badge fills, not long text.

### 2.4 Palette deltas

Keep the component, status, and contrast rules above. These presets alter accent, canvas family, and overall mood only; each needs its own contrast validation.

| Palette | Accent light / dark | Canvas light / dark | Mood |
|---|---|---|---|
| Tinh vân | `#6A4FC9` / `#B9A3FF` | `#F2EFFA` / `#0D0A18` | Tím/lavender, mềm, có sắc hồng ở nền |
| Cực quang | `#287E68` / `#7DDDB9` | `#EDF6F2` / `#0C1718` | Xanh lục ngọc, mát và sinh động |
| Than chì | `#52677A` / `#8AB4FF` | `#E8EBEF` / `#111113` | Trung tính, ít màu, nền tối than |
| Bình minh sương | `#C15F3C` / `#FFAA85` | `#F7EEE9` / `#211512` | Cam đào, ấm, ánh hồng nhạt |
| Ngân hà | `#E8835D` / `#F0946F` | `#08080A` / `#000000` | Tối sâu, điểm sao tĩnh, giữ sao trong pane kính |
| Orrery | `#BD5128` / `#E8835D` | `#F3F3F6` / `#131315` | Nhận diện Orrery, cam trên nền trung tính |
| Console | `#087F96` / `#3DD6F5` | `#E7F0F2` / `#0C0D10` | Kỹ thuật, tương phản cao, mono có tiết chế |

## 3. Typography

Bundle offline `Manrope` cho UI và `JetBrains Mono` cho code/ID. Settings > Text vẫn cho chọn system font. Số đếm và thời gian bật `font-variant-numeric: tabular-nums`. Scale dùng rem 16px; `text-2xs` được sửa thành 11px/14px, `text-xs` giữ 12px/16px, không còn hai token cùng 12px.

| Role | Size / line-height | Weight | Font / behavior |
|---|---:|---:|---|
| Micro metadata | 10 / 13px | 500 | Manrope, chỉ timestamp phụ |
| Label | 11 / 14px | 600 | Manrope, chữ thường, letter spacing 0.01em |
| Rail row title | 12 / 16px | 600 | Manrope, single line ellipsis |
| Rail row detail | 11 / 14px | 500 | Manrope, tabular time |
| Pane header | 12 / 16px | 650 | Manrope |
| Pill tab | 12 / 16px | 600 | Manrope |
| Body compact | 13 / 18px | 450 | Manrope |
| Body standard | 14 / 20px | 450 | Manrope |
| Section title | 16 / 22px | 650 | Manrope |
| Page title | 20 / 28px | 700 | Manrope |
| Code / terminal | 12 / 18px | 400 | JetBrains Mono, ligatures off |
| Code emphasized | 12 / 18px | 600 | JetBrains Mono |
| Numeric KPI | 20 / 26px | 650 | Manrope, tabular nums |
| Keyboard hint | 11 / 14px | 600 | JetBrains Mono |

Không dùng chữ viết hoa toàn bộ cho label; tối đa 1 cấp chữ phụ dưới 12px để rail vẫn quét nhanh.

## 4. Spacing, sizes, shape, elevation

| Token / element | Đề xuất |
|---|---:|
| Grid cơ sở | 4px |
| Spacing steps | 4, 8, 12, 16, 20, 24, 32, 40px |
| Icon-only control | 28x28px, compact 24x24 chỉ trong pane header |
| Control height | 24px compact, 28px small, 32px standard, 36px prominent |
| Chip height | 20px thống nhất cả status, metadata, counter; compact icon-only 20px |
| Rail width | 264px default, 224px min, 360px max, kéo resize theo 4px |
| Collapsed avatar column | 56px, hit target 40px |
| Tab strip | 36px high, tab pill 28px high |
| Pane header | 34px high |
| Explorer drawer | 288px default, 240px min, 400px max; overlay khi chưa pin |
| Pane gap | 8px, divider hit area 12px |
| Shell gap | 12px giữa rail và stage, 8px trong stage |
| Main inset | 12px dọc/ngang, tăng 16px khi viewport >= 1600px |
| Rail section gap | 12px |
| Card internal padding | 12px; compact 8px |
| Radius ladder | window 24px, pane/card 18px, control 12px, inner 8px, pill 999px |
| Hairline | 1px line; active pane accent ring 2px |
| Focus ring | 2px solid focus, offset 2px |
| Layer 0 shadow | none, canvas và surface phẳng |
| Layer 1 shadow | `0 8px 24px -16px rgb(20 50 70 / .22)` light, `0 8px 24px -16px rgb(0 0 0 / .55)` dark |
| Layer 2 shadow | `0 18px 40px -12px rgb(20 50 70 / .35)` light, dark alpha .72 |
| Accent shadow | `0 6px 16px -6px accent / 55%, inset 0 1px 0 white / 22%` |
| Blur chrome | `blur(20px) saturate(1.5)` |
| Blur popover/dialog | `blur(24px) saturate(1.6)` |
| Blur pane | 0px, terminal surface không blur |

Reduced transparency: glass chuyển thành `--ccm-material` opacity 1, bỏ backdrop-filter và thay edge bằng line-strong. Không đổi kích thước hoặc thứ bậc màu.

## 5. Layout

Workspace tính từ viewport, không có AppHeader. Rail head chứa chuyển Workspace/Monitor và logo/app menu; rail foot chứa Usage, History, Inbox, Settings. Nội dung stage nhận phần chiều cao còn lại. Các con số dưới đây là vùng khả dụng gần đúng sau inset 12px, không tính titlebar native Electron.

| Viewport | Rail | Stage | Explorer | Behavior |
|---|---:|---:|---:|---|
| 1920x1080 | 264px | ~1604px | 288px overlay | Rail và stage đủ; Explorer mở overlay, ghim thành cột thì stage còn ~1304px |
| 1600x900 | 264px | ~1284px | 288px overlay | Default; rail không co, tab có thể gộp overflow thành `+N` |
| 1280x800 | 248px | ~984px | 288px overlay | Rail ưu tiên 248px; Explorer overlay; pane tối thiểu 220px mỗi chiều |
| 1024x800 | 56px | ~944px | 288px overlay | Rail collapse thành avatar; label Cần bạn thành badge dot/count; drawer không pin |

Breakpoints: >=1440 giữ rail 264; 1200-1439 rail 248; 1024-1199 rail collapsed 56 mặc định, mở tạm bằng hover/click; <1024 rail thành overlay 248px và stage chiếm toàn bộ. Không tự chuyển thành mobile layout ở 1024. Explorer chỉ pin thành cột khi viewport >=1440 và stage sau pin vẫn >=720px; nếu thấp hơn luôn overlay.

### Màn còn lại

| Màn | Shell và layout đề xuất |
|---|---|
| Monitor | Cùng rail chrome, màn nội dung full height. Summary row trên cùng; segmented Bảng/Quỹ đạo + filter trong toolbar 40px. Bảng 4 cột khi >=1440, 2x2 ở 1024-1439, 1 cột dưới 1024. Cột trống thu còn 40px. Quỹ đạo detail aside 320px từ >=1280, thành drawer dưới mức đó. CommandPanel dock đáy 32px, bảng mở tối đa 40vh. |
| Usage | Trang scroll trong max-width 1440px, padding 24px desktop/16px 1024. KPI cards 4 cột >=1440, 2 cột từ 1024; charts full-width theo hàng, bảng có scroll ngang riêng nếu cần. |
| History | Max-width 1440px. Toolbar sticky 48px với search/filter/date; danh sách + detail split 320px khi >=1440, detail thành drawer dưới 1440. Timeline một cột, không tạo cột trống phụ. |
| Settings | Rail settings nội bộ 220px và content max-width 900px khi >=1280; ở 1024 nav settings thành segmented/select trên đầu. Grouped surface cards, mỗi setting row min-height 48px. Text/palette controls nằm trong Appearance. |
| Inbox | Popover 360px rộng, tối đa 520px cao; dưới 1024 dùng drawer 420px hoặc full width khi <600px. |
| Mission Control | Chế độ phụ của Workspace, overlay toàn stage. Toolbar 40px, card grid min 280px, max 720px, gap 12px. Escape quay về pane focus trước đó. |

## 6. Component specs

State chung: hover dùng fill 7% ink; pressed dùng fill-strong 12%; focus-visible dùng ring 2px; disabled giảm tương phản nền, text vẫn dễ đọc cho tên control nhưng action rõ disabled bằng icon/shape, không dùng opacity thấp để truyền nghĩa.

| Component | Size / padding / radius | Fill, edge, shadow | States |
|---|---|---|---|
| Rail row | min-height 48px; padding 8px 10px; radius 12px | surface trong suốt, line trái 2px accent khi focused | 1-line ưu tiên title + status/time; 2-line thêm tool/model + context gauge. Hover fill; active accent-soft + accent-ink; waiting có glyph + label; error dùng error-soft; ended muted; selected multi-select outline accent. |
| Cần bạn card | 44px collapsed summary; expanded item 64px; padding 10px; radius 14px | waiting-soft, edge waiting/25%, shadow layer 1 | Title 1 dòng, reason/tool và wait time dòng 2; action “Mở” 28px. Hover edge waiting; focus ring; empty state thu gọn, không giữ card rỗng. |
| Pill tab | 28px height, padding 8px, radius 999px | inactive transparent; active raised + shadow-control | Hover fill; active ink + small agent/project monogram; waiting badge uses shared chip; close icon appears hover/focus, remains accessible. |
| Pane card | Radius 18px; inner padding 0 | raised glass 60%, edge line, shadow layer 1; active ring 2px accent | Active accent ring, header tint accent-soft 5%; inactive keeps ink full contrast; resize keeps blur static; pane terminal body opaque well. |
| Pane header | 34px, horizontal padding 10px, radius inner 8px | transparent, active head tint accent-soft | Title 12px; provider glyph 16px; actions 24px hit area; hover control fill; disabled ink-faint, tooltip. |
| Chip | 20px high, horizontal padding 7px, radius 999px | role soft tint, 1px transparent/edge optional | One shared geometry for status/meta/count; status has glyph 12px + label; hover only if actionable; focus ring if interactive. |
| Button primary | 32px, padding 12px, radius 12px | accent gradient, on-accent, accent shadow | Hover brightness +3%; pressed scale .98/180ms; focus ring; disabled solid neutral fill + ink-faint. |
| Button secondary | 32px, padding 12px, radius 12px | control 72%, line, shadow-control | Hover control-hover; pressed fill-strong; focus ring; disabled no shadow. |
| Button quiet / destructive | 28-32px, radius 10px | transparent; destructive uses error text/soft | Hover fill; active soft fill; destructive confirmation for irreversible delete only. |
| Segmented | 32px outer, 28px thumb, radius 999px | fill 7%, thumb white/.9 light or white/.13 dark | Selected thumb shadow; keyboard arrows; focus ring on group; disabled item ink-faint. |
| Input / select | 32px, horizontal 10px, radius 10px | control, line, shadow-control | Hover line-strong; focus 2px focus + subtle glow; invalid error edge and inline message; disabled fill muted, placeholder faint. |
| Menu | 220-280px width, row 32px, padding 4px, radius 14px | glass-pop, glass edge, layer 2 shadow, blur 24px | Hover selected row accent-soft; checked glyph; separators line; keyboard focus follows selection; disabled item cannot activate. |
| Dialog | 480px default, max 640px, padding 20px, radius 20px | glass-pop, glass edge, layer 2 shadow, blur 24px; scrim `rgb(20 40 50 / .28)` light / black .5 dark | Entrance opacity + translateY 8px; focus trap; Escape closes unless blocking task; primary action right aligned. |
| Toast | 360px max, min-height 48px, padding 12px, radius 14px | dark ink surface light / pale surface dark, layer 2 | Status icon and title, optional one-line detail, action button; 4s default, persistent for error/action; no blur over terminal text. |
| Ctrl+K palette | 600px width, max 720px, top 18vh, radius 20px | glass-pop, edge, layer 2, blur 24px | Search input 44px; results 40px rows; selected accent-soft; grouped sections; keyboard up/down/enter/esc; show shortcut in kbd. |
| Tooltip | max 320px, padding 6px 8px, radius 8px | glass-pop, line, layer 2, blur 20px | Delay 450ms, instant for keyboard focus; never hide focused content; 11px text, contrast AA. |

Rail row content: first line is project/session title, max one line. Second line chooses exactly one highest value detail: current tool + elapsed time while working, wait reason + elapsed while waiting, model + latest result when idle. Context gauge is a compact 3px bar at trailing edge or second-row trailing slot; do not stack multiple metrics.

## 7. Motion

| Motion | Duration / easing | Behavior |
|---|---|---|
| Hover / color | 120ms, `cubic-bezier(.22,1,.36,1)` | Fill, line, shadow, text color |
| Button press | 100ms in / 180ms out | Scale to .98, no spring bounce |
| Drawer / rail | 220ms, `cubic-bezier(.22,1,.36,1)` | Translate + opacity, keep pane canvas stationary |
| Menu / dialog | 180ms, same ease | Opacity + 6-8px vertical offset |
| View switch | 160ms crossfade | Avoid moving terminal panes during view switch |
| Status arrival | 450ms one-shot | Soft ring or edge sweep for waiting/error; never pulse indefinitely |
| Background | none | Static gradients; no parallax, no animated blur |

`prefers-reduced-motion: reduce`: all transitions <=1ms, remove transforms, sweeps, pulses, orbit rotation; preserve immediate selected/focus states and status glyphs. User setting “Reduce transparency” also disables blur but does not imply reduced motion.

## 8. Thay đổi so với UI hiện tại

| Nhóm | Hiện tại | Đề xuất |
|---|---|---|
| Canvas | Orrery neutral `#ECECEE` light, `#1E1E1E` dark | Sương gradient tĩnh xanh ngọc, lam, đào; palette mới tách khỏi classic |
| Surface | panel khá đặc, shape thay theo theme | Glass phân tầng theo elevation, solid fallback cho reduce transparency |
| Accent | Apple blue mặc định, Orrery cam | Teal `#2A7F9E` sáng và cyan `#73C6DC` tối |
| Status | status token dựa Apple orange/yellow/green/red | working/waiting/error/done màu ấm rõ, ended dùng ink-faint; glyph + nhãn bắt buộc |
| Typography | System UI, `text-2xs` và `text-xs` gần nhau | Manrope + JetBrains Mono bundle; 11px và 12px phân cấp rõ |
| Chip | h20 status, một số chip h khác nhau | Một geometry h20, cùng padding/radius/icon alignment |
| Rail | 240px default, 200-560, avatar 52px | 264px default, 224-360; avatar column 56px; live session summary 2 dòng |
| Navigation | AppHeader 52-72px, nav và trạng thái trên header | Bỏ AppHeader ở next UI; Workspace/Monitor ở rail head, Usage/History/Inbox/Settings ở rail foot |
| Workspace | rail + stage + Explorer 280px thành cột | Rail + “Cần bạn” + pill tabs; Explorer 288px drawer, pin thành cột khi đủ rộng |
| Tab | 28px tab row, tab kiểu cũ | Strip 36px, pill 28px, overflow `+N` |
| Pane | header 28px, card radius 12px, well theo theme | header 34px, card radius 18px, active ring 2px, well luôn tối và không blur |
| Mission Control | chế độ phủ canvas hiện tại | Chế độ phụ có cùng glass card system, vào/ra rõ ràng |
| Màn Monitor | dùng header chung, board/orbit hiện có | Chung shell mới, 4 cột responsive, aside Quỹ đạo thành drawer khi hẹp |
| Usage / History | trang cuộn max-width 1560px, header chung | max-width 1440px, KPI/table responsive, nav thống nhất trong rail |
| Settings | header chung và nhóm settings | shell rail, nav settings phụ, font/palette/reduce transparency được gom trong Appearance |
| Motion | nhiều animation từ UI hiện tại | motion scale thống nhất, signal one-shot, reduced-motion bỏ chuyển động phi chức năng |

## 9. Ghi chú khi dựng mockup

- Dựng trước Workspace ở 1600x900 light và dark, sau đó kiểm tra 1280x800 và 1024x800 để xác nhận collapse.
- Rail demo tối thiểu 4 trạng thái: working, waiting, idle, error; “Cần bạn” có item thật và trạng thái rỗng.
- Pane demo cần tab active/inactive, Claude/Codex/Shell, terminal well, split divider, Explorer overlay và Explorer pinned.
- Không vẽ thông tin model hoặc tool nếu dữ liệu chưa có; vùng dữ liệu thiếu để trống theo behavior trong plan.
- Tên palette tiếng Việt theo plan; palette preset chỉ thay token màu, không đổi spacing/component geometry.
