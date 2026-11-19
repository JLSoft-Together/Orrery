# WP-68a report: mockup web app iPhone / iPad theo "Sương kính"

Trạng thái: **xong mockup, chờ chủ nhân duyệt**. Không sửa `apps/web/**`, `apps/pwa/src/**`. Không commit.

## Mở xem

- So sánh cũ | mới: `docs/qa/v98/WP-68a/compare.html` (14 màn; mỗi màn: iPhone cũ | mới sáng | mới tối, iPad cũ | mới; mục mở rộng có 6 biến thể WebKit).
- Mockup sống: `docs/qa/v98/WP-68a/mockup/index.html`. Thanh trên chọn bảng màu (Sương kính / Tinh vân / Ngân hà), sáng / tối, Giảm trong suốt, thiết bị. Không JS: bộ chọn chạy bằng `:has()`. Không `style=""`, không `<style>`, không `<script>`. Font Manrope + JetBrains Mono nằm trong `mockup/fonts/` (OFL, copy từ `@fontsource-variable`).
- Dựng lại: `node docs/qa/v98/WP-68a/tools/build.mjs` (sinh `index.html` từ `tools/{icons,parts,screens}.mjs`), `node .../tools/shoot.mjs` (ảnh), `node .../tools/compare.mjs` (trang so sánh). Chromium dùng `channel: msedge` (máy không có Chromium của Playwright), đổi bằng env `PW_CHANNEL`.

## Danh sách ảnh (`docs/qa/v98/WP-68a/`, JPEG q88)

Tên: `<màn>-<phone|ipad>-<biến thể>-<webkit|chromium>.jpg`. iPhone 390×844 chụp @2x (780×1688), iPad 820×1180 chụp @1x.

| Màn (`<màn>`) | Nội dung |
|---|---|
| `home` | Home nhiều máy: 2 máy đang nối (Relay, LAN) + 1 máy mất kết nối, hạn mức theo máy |
| `pair` | Ghép: Quét QR / Nhập mã; iPad hiện cả hai + tiến trình LAN → relay |
| `lock` | Bước "Bật khoá Face ID?" ngay sau khi ghép |
| `monitor` | Monitor A: session lùi vào trong project, lọc trạng thái, segmented Project / Trạng thái |
| `limit` | Banner lỗi `limit` (role=alert, Thử lại), dữ liệu cũ mờ 62% |
| `detail` | Chi tiết session: model, context, câu hỏi đang chờ, sub-agent, hoạt động |
| `ws-a` | Workspace A: tab pill, chip pane, pane xem trước + Mở toàn màn hình |
| `term` | Terminal: thanh hỏi quyền, terminal đặc, dock phím + ô gõ đặc |
| `ws-b` | Workspace B: lưới pane, viền accent pane đang chọn |
| `files` / `diff` / `git` / `run` | Viewer Tệp / Diff / Git commit (+ lịch sử trên iPad) / Chạy |
| `settings` | Cài đặt, mục Giao diện mới (bảng màu, sáng / tối, Giảm trong suốt) |

Biến thể: WebKit `mist-light`, `mist-dark`, `nebula-light`, `nebula-dark`, `galaxy-dark`, `mist-light-reduced` (Giảm trong suốt); Chromium `mist-light`, `mist-dark`. Tổng 224 ảnh, ~23 MB.

**Lưu ý chụp:** Playwright WebKit trên Windows không vẽ `backdrop-filter` (CSS.supports trả `true` nhưng ảnh không mờ). Ảnh WebKit vì vậy là trường hợp xấu nhất (chỉ có lớp trong suốt), ảnh Chromium mới có blur. Blur thật trên iPhone phải kiểm trên máy ở WP-68b.

## Quyết định thiết kế

1. **Token y hệt desktop next**: giá trị lấy nguyên từ `apps/web/src/next/{tokens,palettes}.css` (mist mặc định; mockup trình diễn Sương kính, Tinh vân, Ngân hà; 5 bảng còn lại dùng được ngay vì chỉ đổi biến). Trạng thái = icon + chữ + màu; working xanh dương, chờ bạn vàng hổ phách như desktop.
2. **Glass chỉ ở chrome**: thanh trên, tab bar, rail iPad, sheet / popover, thẻ khoá trên iPad. Thẻ danh sách dùng nền kính trong suốt nhưng **không blur** (giống `mist-card` desktop). Terminal, code, diff, log: nền `--ccm-well` đặc. Dock phím + ô gõ dưới terminal: `--ccm-material` đặc (không blur trên vùng terminal đang chạy chữ).
3. **Thanh trên / tab bar điện thoại đậm hơn desktop**: token mới `--ccm-bar` (sáng 72% trắng, tối 80% `--ccm-material`). Lý do: nội dung cuộn ngay dưới thanh; ảnh WebKit (không blur) cho thấy với glass 5% của desktop, chữ dưới đè lên nhãn tab. 72% vẫn thấy màu sương, đọc được cả khi blur không chạy.
4. **Tab bar dạng viên nổi** (cách lề 16, đáy 26), 3 mục giữ nguyên (Giám sát / Workspace / Cài đặt), mục đang chọn là pill `--ccm-accent-soft`, badge số "chờ bạn" trên Giám sát.
5. **Home**: nút Ghép máy chuyển từ FAB lên thanh trên (nút `+` accent) vì FAB đè thẻ máy cuối. Workspace giữ FAB "Pane mới".
6. **iPad đồng bộ desktop v103**: rail kính 300 px bên trái thay cho cột master + tab bar dưới: đầu rail = máy + route chip, segmented Giám sát / Workspace, Project / Trạng thái; thân = session lùi vào trong project (Workspace: tab → pane); **chân rail = hạn mức** (như footer v103) + hàng icon. Cài đặt trên iPad: rail mục + trang bên phải.
7. **Row 2 dòng như rail desktop (R1)**: dòng 1 icon trạng thái + tên + thời gian; dòng 2 agent màu, model, trạng thái, chi tiết; gauge context 28×4, vàng khi ≥ 65%.
8. **Lưới pane**: chip trạng thái xuống dưới tên (pane hẹp 170 px không đủ chỗ cho tên + chip + menu).
9. **Settings → Giao diện** (mới trên web app): 6 thẻ bảng màu có mini preview, Sáng / tối thành segmented (thay dropdown), công tắc Giảm trong suốt kèm giải thích pin / cuộn mượt.
10. **Font**: Manrope + JetBrains Mono bundle như desktop (web app hiện dùng font hệ thống).
11. Bỏ toàn bộ nhãn mono viết hoa (`ĐANG KẾT NỐI`) của web app cũ, dùng nhãn sans 13/700 như next (`--ccm-label-case: none`).

## Chi phí GPU dự kiến trên iPhone và cách giảm

- **Mỗi màn tối đa 2 lớp blur** (thanh trên + tab bar); terminal / viewer chỉ 1 (thanh trên); sheet thêm 1 lớp tạm, nền sau có scrim nên đứng yên. iPad 2 lớp (rail + thanh trên), rail blur nền sương tĩnh nên cuộn rail không làm mờ lại phần sau.
- **Ước lượng**: thanh trên 390×99 + tab bar 358×64 ≈ 61 nghìn px CSS ≈ 550 nghìn px máy (@3x) phải blur lại mỗi frame khi cuộn. A15 trở lên dư sức 60 fps; A12–A13 (iPhone XR / 11) có thể rớt frame khi cuộn nhanh danh sách dài.
- **Cách giảm** (đã áp trong mockup):
  1. Không blur thẻ, danh sách, terminal, code, log; dock dưới terminal đặc.
  2. Thanh dùng `--ccm-bar` 72–80% nên đề xuất giảm blur điện thoại xuống `blur(16px) saturate(1.4)` mà không mất chữ.
  3. Không đổi / animate blur khi cuộn (không bật tắt `backdrop-filter` theo scroll: tạo lại layer còn tốn hơn).
  4. Nền sương: iOS bỏ qua `background-attachment: fixed` → dùng 1 lớp `position: fixed` (pseudo-element) vẽ 1 lần, không repaint khi cuộn.
  5. **Giảm trong suốt** (`data-glass="reduced"` như desktop): mọi lớp thành `--ccm-material` đặc, `--ccm-glass-blur: none`. Safari chưa có `prefers-reduced-transparency` → công tắc tay trong Cài đặt, nhớ `localStorage`.
  6. Đo ở WP-68b trên iPhone thật: Safari Web Inspector → Timelines → Rendering Frames, cuộn Monitor 30 session + terminal đổ 1000 dòng/s, mục tiêu ≥ 55 fps; nếu không đạt trên máy cũ thì bỏ blur ở tab bar trước.

## Đề xuất dùng chung token với desktop

**Đề xuất: tách package `packages/mist-tokens` (`@ccm/mist-tokens`)**, không copy:

- `src/tokens.css` = phần biến của `apps/web/src/next/tokens.css` (dòng 1–261: biến màu / radius / motion, `data-glass='reduced'`, `data-motion`), giữ nguyên selector `:root[data-ui='next']`. Phần component (`.glass`, `.mist-*`, dòng 262+) ở lại `apps/web`.
- `src/palettes.css` = `palettes.css` trừ rule `.ccm-panel` của Ngân hà (component). `src/palettes.ts` = `palettes.ts` (dùng cho thẻ chọn bảng màu ở cả hai app).
- Web app chỉ cần đặt trên `<html>`: `data-ui="next"`, `data-next-theme`, class `dark`, `data-glass` → selector chạy nguyên, không viết lại. Thêm lớp riêng mobile (`--ccm-bar`, blur 16px) trong `apps/pwa/src/styles/`.
- Test: chuyển `lib/themes.test.ts`-kiểu contrast vào package (mọi palette × sáng / tối, ghép trên `--ccm-bar` và `--ccm-glass`), + 1 test ở pwa import được token.
- **Thứ tự**: làm sau khi WP-70b (đang sửa `apps/web/**`) merge, để không xung đột; desktop đổi 2 dòng import.
- Phương án dự phòng (nếu không muốn package): copy vào `apps/pwa/src/styles/mist/` + test so tập biến / giá trị với file desktop (fail khi lệch).

## CSP relay (`style-src 'self'`, không inline)

- Mockup đã đúng luật: CSS file ngoài, font `'self'`, không `style=""`, icon SVG sprite `<use href="#…">`.
- WP-68b cần giữ: CSS do Vite xuất thành file (không inline critical CSS); `:style` của Vue gán qua CSSOM (`el.style.setProperty`) nên CSP cho phép, nhưng **không** để `style="…"` trong `index.html` / template tĩnh được render thành HTML string; xterm đã xử lý ở `terminal/xtermStyles.ts`. Font Manrope / JetBrains Mono vào precache SW (`font-src 'self'` sẵn có). Không cần đổi CSP relay.
- `color-mix()`, `:has()`, `backdrop-filter` không prefix cần iOS 16.4+ / Safari 18; luôn kèm `-webkit-backdrop-filter`.

## Câu hỏi cho chủ nhân

1. **Hạn mức** (Claude Code 5h / tuần, Codex) trên thẻ máy và chân rail iPad: desktop chưa gửi số này cho điện thoại. Thêm vào snapshot remote ở WP-68b (con đề xuất, cùng nguồn với footer v103), hay bỏ khỏi web app?
2. **Bảng màu**: điện thoại chọn riêng (con đề xuất, lưu trên máy) hay tự theo bảng màu đang dùng trên máy tính?
3. **Font**: bundle Manrope + JetBrains Mono (+~110 KB precache, giống desktop) hay dùng SF Pro của iPhone?
4. **Tab bar viên nổi** (như iOS mới) hay thanh tràn viền như bản cũ?
5. **iPad bỏ tab bar dưới**, dùng rail trái giống desktop: đồng ý?
6. **Giảm trong suốt** mặc định tắt (con đề xuất), hay bật sẵn và để người dùng bật kính?
7. Tách token sang `packages/mist-tokens` sau khi WP-70b merge: đồng ý?

## Ước lượng WP-68b (làm thật)

| Gói | Nội dung | Model | Cỡ |
|---|---|---|---|
| 68b-1 | `packages/mist-tokens` + desktop đổi import + contrast test; PWA root attrs, fonts, lớp mobile (`--ccm-bar`), nền sương fixed, Settings → Giao diện | opus thiết kế / sonnet làm | ~1 ngày agent |
| 68b-2 | Home, Ghép, Khoá, Monitor A/B, chi tiết, banner limit / mất kết nối / trimmed, tab bar mới | sonnet | ~1 ngày |
| 68b-3 | Workspace A/B, Terminal (PermissionBar, KeyBar, InputBar), 4 viewer + editor, sheet / confirm / Undo bar; iPad rail | sonnet | ~1–1,5 ngày |
| 68b-4 | QA: E2E pwa 80 bước giữ testid, ảnh cả bộ, đo FPS trên iPhone thật | sonnet + chủ nhân cầm máy | ~0,5 ngày |

Hạn mức (nếu chủ nhân chốt có) thêm ~0,5 ngày phía desktop (`core-remote` snapshot) + test.

## File đã tạo

- `docs/qa/v98/WP-68a/mockup/{index.html,tokens.css,mist.css,fonts/*.woff2}`
- `docs/qa/v98/WP-68a/tools/{icons,parts,screens,build,shoot,compare}.mjs`
- `docs/qa/v98/WP-68a/{compare.html,compare.css}`, 224 ảnh `*.jpg`
- Báo cáo này.
