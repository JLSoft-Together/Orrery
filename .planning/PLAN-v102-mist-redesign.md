# v102: Giao diện mới Sương kính (UI next gen)

Trạng thái: **plan + spec đã chốt (15/11), chưa code**, Q1 chốt kính, không flat (SPEC mục 10). Chủ nhân đang làm v100 (click menu) + v101 (disk manager) trên `feat_clean_memory`.

## Tóm tắt
Thay toàn bộ UI Orrery theo theme Sương kính của jin-tools: kính mờ, nền sương gradient tĩnh, Manrope.
Workspace là khung làm việc chính nên bố cục xoay quanh nó (phương án **W2**). Giao diện cũ giữ nguyên
song song sau cờ; bản đầu mặc định vẫn là giao diện cũ, có nút **Thử giao diện mới (UI next gen)**.

## Nguồn
- jin-tools: `apps/web/src/styles/tokens.css` (`mist`), `themes.css`, `main.css` (`@utility glass`), `layouts/AppShell.vue`; ảnh `F:\work\tools\jin-tools\.jin\logs\theme-qa\mist-*.png`.
- Mock: token trên bố cục cũ https://claude.ai/artifact/LgvYzVtj2K9eoTFeMiFFQV ·
  5 hướng toàn app https://claude.ai/artifact/BS4X5TQC1fKUjJdqTghiqm ·
  **4 phương án Workspace (chọn W2)** https://claude.ai/artifact/3toYqxZo6vAA5n7DyNTEAp

## Quyết định đã chốt (15/11/2026)
| # | Quyết định |
|---|---|
| 1 | Thay toàn bộ UI (bố cục + component), kèm design system đầy đủ |
| 2 | Bố cục Workspace = **W2**: rail session sống + hàng đợi "Cần bạn", Explorer là ngăn kính trượt đè lên pane (ghim lại thành cột được), Mission Control là chế độ phụ, Ctrl+K kính |
| 3 | Blur `backdrop-filter: blur(20px) saturate(1.5)` như jin-tools. Bỏ rule "No blur" 09/11 khi code (sửa CLAUDE.md + `COLOR-SYSTEM.md`) |
| 4 | Nền sương radial-gradient tĩnh: ngoại lệ thứ 2 của rule "no gradients" |
| 5 | Font mặc định Manrope + JetBrains Mono bundle offline (`scripts/gen-fonts.mjs`); font hệ thống vẫn chọn được ở Settings → Text |
| 6 | Bảng màu UI mới (chọn lọc): **Sương kính** (mặc định, sáng + tối), Tinh vân, Cực quang, Than chì, Bình minh sương (sáng + tối), Ngân hà (tối, giữ sao), Orrery (sáng + tối), Console (tối). Apple, Xcode, GitHub, Solarized, Monokai, Dracula, Material, One Dark, Islands chỉ còn ở giao diện cũ |
| 7 | 2 nút chuyển: giao diện cũ có "Thử giao diện mới (UI next gen)", giao diện mới có "Về giao diện Orrery cũ". Đổi ngay, không restart |
| 8 | Bản đầu **mặc định giao diện cũ**; đổi mặc định sang mới ở bản sau, khi chủ nhân thấy ổn |
| 9 | **Làm hết mọi màn rồi mới ra bản** (không ra nửa cũ nửa mới) |
| 10 | Code trong **worktree riêng** nhánh `feat_mist_ui` từ `master`, làm song song với `feat_clean_memory` |
| 11 | Chủ nhân duyệt **toàn bộ chức năng mới** đề xuất sau phân tích UI: rail 2 dòng + model, nhóm Project / Trạng thái, Cần bạn + taxonomy + hoãn, Ctrl+K nhóm Session, responsive, Settings Giao diện mới, motion 3 mức, 11 góp ý kéo thả / split. Chi tiết + mã ID: **`SPEC-v102-mist-ui.md`** |
| 12 | Phím mới duy nhất: Tách sang tab mới Ctrl+Alt+T (macOS Alt+Cmd+T). Mọi thao tác khác dùng phím có sẵn trong `lib/shortcuts.ts`. Hoàn tác bố cục không gắn Ctrl+Z |
| 13 | Mockup chỉ deploy subdomain jinvu.vn (`orrery-ui.jinvu.vn`), không tạo artifact |

## Thiết kế kỹ thuật

### Cờ giao diện
- `ccm.uiGeneration` = `classic` (mặc định) | `next`; root `data-ui="classic|next"`; `stores/settings.ts` + `index.html` `pick` đọc trước khi mount (không chớp).
- Palette của UI mới lưu riêng `ccm.nextTheme` (không đè `ccm.theme` của giao diện cũ), quay lại cũ là y nguyên như trước.
- **Bất biến:** `classic` render đúng như 4.0.5 (E2E hiện có chạy với `classic` phải pass không sửa).
- **Bất biến (chủ nhân hỏi 15/11):** UI mới giữ đủ kéo thả + split: kéo pane để split / dock / swap (drop preview), kéo tab đổi chỗ / kéo ra khỏi dải, kéo row rail giữa tab, kéo project đổi thứ tự, divider kéo + phím mũi tên (ratio 15-85%), zoom, undo đóng, pane không remount. Dùng chung `splitLayout`, `useRailDrag`, `view/geometry.ts`; chỉ đổi lớp vẽ (drop preview kính, ghost pill). E2E kéo thả chạy cả `classic` và `next`. Danh sách đầy đủ: `ui-inventory/01-workspace.md` mục Must keep 4, 6.

### Tách code cũ / mới
- Shell + bố cục khác hẳn → component mới trong `components/next/` (vd `next/NextShell.vue`, `next/rail/LiveRail.vue`, `next/ExplorerDrawer.vue`), `App.vue` chọn theo `data-ui`.
- Component lá dùng chung (dialog, menu, chip, button, viewer, TermPane…) → **một component**, đổi giao diện bằng token dưới `:root[data-ui='next']`. Chỉ tách khi cấu trúc DOM phải khác.
- Logic (stores, `lib/rail/*`, `splitLayout`, `usePaneInfo`, keys) dùng chung 100%, không copy.

### Token (design system)
- File mới `apps/web/src/next/tokens.css` + `next/palettes.css`; tên token giữ prefix `--ccm-*` để component lá không phải sửa.
- Thêm token: `--ccm-glass`, `--ccm-glass-pop`, `--ccm-glass-blur`, `--ccm-glass-edge`, `--ccm-side`, `--ccm-nav-on`, `--ccm-bg-image`, `--ccm-accent-grad`, `--ccm-shadow-accent`, radius `window 24 / card 18 / control 12 / inner 8`.
- Blur chỉ đặt ở chrome + lớp nổi (rail, tab strip, Explorer drawer, popover, menu, dialog, Ctrl+K, toast). **Pane terminal không blur**: nền gradient mịn nên panel trong suốt nhìn như kính, không tốn GPU khi resize / cuộn xterm (giống jin-tools: list dài không blur).
- Terminal well: xanh đen trong 92% (tối 55%), xterm `allowTransparency` (đã có từ Stars in panels).
- Contrast test kiểu `lib/themes.test.ts` cho mọi palette, tính trên surface ghép canvas (WCAG AA ≥ 4.5).
- `prefers-reduced-transparency` / Settings "Giảm trong suốt": panel đặc, tắt blur.

### Workspace W2
- **Rail sống** (264px, kéo được, thu thành cột avatar 56px có vòng trạng thái): dòng 1 tên + chip trạng thái (+ thời gian khi working); dòng 2 tool đang chạy / kết quả gần nhất + context gauge. Nguồn: `usePaneInfo.activityOf`, `railStateOf`, `ContextGauge`.
- **Cần bạn**: thẻ đầu rail từ `lib/rail/attentionList.ts`, nút Mở → `nextAttention`.
- Giám sát / Workspace thành segmented đầu rail; Hạn mức, Inbox, Thông báo, Cài đặt về chân rail. Không còn `AppHeader` ở UI mới.
- Tab strip pill + monogram tone project; chip "cần bạn" trên tab.
- **Explorer drawer** 270px trượt đè từ phải (Ctrl+Shift+E), nút ghim → cột cố định như cũ; nhớ trạng thái `ccm.next.explorerPinned`.
- Pane: card kính bo 16, header 34px, viền active 2px accent, group frame kính viền mảnh.
- Mission Control: chế độ phụ (phím hiện có), thẻ kính.

## Gói việc (manager, worktree `feat_mist_ui`)
| WP | Nội dung | Phụ thuộc | Model |
|---|---|---|---|
| WP0 | Worktree + cờ `uiGeneration`, `data-ui`, 2 nút chuyển, E2E helper chạy 2 chế độ | – | sonnet |
| WP1 | Design System: token, 9 palette, bundle Manrope, contrast test; artifact Design System | WP0 | opus |
| WP2 | Shell mới: nền sương, rail head / foot, bỏ header, routing view | WP1 | sonnet |
| WP3 | Rail sống + Cần bạn + cột avatar | WP2 | sonnet |
| WP4 | Canvas: tab strip, pane head, group frame, well, divider, zoom, dim | WP2 | sonnet |
| WP5 | Explorer drawer + ghim + phím | WP2 | sonnet |
| WP6 | Lớp nổi: Ctrl+K, QuickOpen, ProjectSwitcher, menu, dialog, toast, popover | WP1 | sonnet |
| WP7 | Mission Control | WP4 | sonnet |
| WP8 | Viewer: file editor, Changes / diff, Git Log, Run, Browser, XML / Flutter preview | WP4 | sonnet |
| WP9 | Monitor (Quỹ đạo, card, activity), Usage, History, Inbox | WP2 | sonnet |
| WP10 | Devices, Extensions, Disk (v101), Settings (mục Giao diện + chọn palette), Office chỉ khung | WP2 | sonnet |
| WP11 | QA: ảnh mọi màn × palette × sáng/tối, E2E 2 chế độ, đo GPU / FPS 6 pane khi resize, cập nhật CLAUDE.md, docs, `.code_index` | tất cả | opus |

Wireframe đủ màn (WP1 kèm theo) cho chủ nhân duyệt trước WP8–WP10.

Map SPEC → WP: WP1 = mục 9 + M2 (token, primitive `GlassDialog` / `GlassMenu` / `GlassTooltip` / `PaneHeader` / chip / gauge) + G4 khoá lưu; WP2 = S1-S4, M1, M6; WP3 = R1-R8, M3; WP4 = T1-T5, D1-D11, M4, M7; WP5 = E1-E3; WP6 = K1-K3 + dialog / menu / toast; WP7 = Mission (thẻ trả lời menu v100); WP10 = G1-G3; WP11 kiểm M5 + đo GPU. Gói WP4 lớn nhất: tách WP4a (tab, pane card, header, click menu) và WP4b (kéo thả + split D1-D11).

## Rủi ro
- **Conflict với v100 / v101**: v101 thêm view Disk (`stores/ui.ts`, `App.vue`, `AppHeader.vue`, `CommandPalette.vue`). Rebase `feat_mist_ui` lên master sau khi 2 phần kia merge; WP10 làm Disk sau cùng.
- **GPU**: blur nhiều lớp + WebGL xterm. Đo ở WP11; vượt ngưỡng thì giảm blur ở rail.
- **Gấp đôi bề mặt test**: mọi E2E chính chạy cả `classic` và `next`.
- **Giao diện cũ bị đụng**: token mới chỉ dưới `[data-ui='next']`; diff ảnh classic trước / sau phải trùng.

## Chốt thêm (15/11/2026)
- Dòng 2 rail **hiện tên model** (Opus / Sonnet / Haiku, Codex model) cạnh tool đang chạy; thiếu dữ liệu thì bỏ trống, không đoán.
- Bảng màu Ngân hà ở UI mới **giữ sao trong panel kính** (dùng lại `--ccm-sky-pane`, theo Settings "Sao trong panel").

## Phân tích UI (15/11/2026)

Tổng hợp ở `.planning/ui-inventory/00-SUMMARY.md` (7 file nguồn 01-07, có số đo thật và đề xuất Codex). Chốt thêm:
- Terminal: mặc định đặc, Settings có công tắc "Terminal trong suốt" (thay mục Terminal well ở trên).
- Rail: segmented "Project | Trạng thái", nhớ `ccm.next.railGroup`; "Cần bạn" sticky ở cả hai.
- Settings giữ dialog, đổi sang kính.
- Motion: setting Đầy đủ / Giảm / Tắt (`data-motion`), bỏ `ccm-meteor-pane` ở UI mới, tín hiệu 1 lần 450ms, tan bụi 700ms.
- Mockup độ chi tiết cao (6 màn + bản flat): https://orrery-ui.jinvu.vn (Worker `orrery-mist`). Nguồn `.planning/mockups/v102/` (`src/*.dc.html`); build `node .planning/mockups/v102/build-site.mjs .planning/mockups/v102`, deploy `cd .planning/mockups/v102/site && npm exec --yes --package=wrangler@4.162.0 -- wrangler deploy`. Không tạo artifact nữa (chủ nhân 15/11), chỉ deploy subdomain jinvu.vn.
- Số đo mới thay số trong mục Workspace W2: rail 264 (224-360), avatar 56, tab strip 36 / pill 28, pane head 34, Explorer 288.

## Release

4.0.6 = v100 click menu + v101 Ổ đĩa + sửa lỗi Tauri + v102, release 1 lần khi cả 3 xong. v102 xong thì thêm mục en + vi
và dòng "Changelog chờ" vào `.planning/RELEASE-4.0.6.md`; không bump version / `changelog.ts` trước khi chủ nhân bảo.

## Điều phối với luồng song song

**Chốt 15/11/2026:** chờ luồng v100 / v101 (click menu + quản lý memory / ổ đĩa) xong và merge master rồi mới bắt đầu
worktree `feat_mist_ui`. Lúc đó thêm gói **WP12**: làm bản UI mới cho các màn / tính năng luồng đó vừa thêm
(click menu dưới pane + hover dòng, Settings → Workspace switch, Settings → Ổ đĩa, view Ổ đĩa và các tab con,
màn khác phát sinh). Liệt kê màn từ diff master lúc bắt đầu, wireframe cùng đợt WP1.
Danh sách màn do luồng v100/v101 cung cấp: `.planning/UI-CHANGES-v100-v101.md` (nhánh `feat_clean_memory`, base 6d7054be).
Ổ đĩa có 9 tab (Duyệt, Tìm kiếm, File lớn, File cũ, Dọn dẹp, File trùng, Sắp xếp, App, RAM) + dialog chuyển ổ; Mission card trả lời menu;
Explorer menu "Chuyển sang ổ khác". Ảnh QA: `docs/qa/v100/WP-0b..0e/`, `docs/qa/v101/`.

Xem `.planning/COORD-2026-11-15-v100-v102.md` (thứ tự merge, namespace i18n, hot file).
