# SPEC v102: UI next gen Sương kính

Chốt 15/11/2026 (chủ nhân duyệt toàn bộ đề xuất trong `ui-inventory/00-SUMMARY.md` mục 3, 4, 4b và mockup https://orrery-ui.jinvu.vn).
Plan và gói việc: `PLAN-v102-mist-redesign.md`. Số đo gốc, file:line: `ui-inventory/01..07`.

Quy ước: **giữ nguyên** = hành vi như 4.0.5; **mới** = chức năng thêm; **đổi** = hành vi khác bản cũ. Mọi chức năng mới chỉ có ở `data-ui="next"`, giao diện cũ không đổi.

## 0. Bất biến

1. `classic` render đúng như 4.0.5, E2E hiện có pass không sửa.
2. Logic dùng chung: stores, `lib/rail/*`, `splitLayout`, `usePaneInfo`, `useRailDrag`, `view/geometry.ts`, `lib/workspaceKeys.ts`. Không copy.
3. Kéo thả + split giữ đủ: kéo pane split / dock / swap, kéo tab, kéo row rail, kéo project, divider kéo + phím (15-85%), zoom, mở lại pane đã đóng, pane không remount.
4. Trạng thái = icon + chữ + màu, không dùng màu đơn độc. Chữ đạt WCAG AA 4.5 trên nền kính đã ghép.
5. Không animate blur / layout; chỉ `transform` + `opacity`.
6. Mọi chuỗi qua vue-i18n namespace `mist.*` (en + vi).

## 1. Shell

| ID | Chức năng | Loại | Chi tiết |
|---|---|---|---|
| S1 | Bỏ AppHeader + footer ở UI mới | đổi | Đầu rail: logo + segmented Workspace / Giám sát. Chân rail: hạn mức 5h / tuần, Inbox (badge), Usage + History, Cài đặt |
| S2 | Nền sương | mới | 3 radial-gradient tĩnh theo palette; Ngân hà dùng sao |
| S3 | Responsive | mới | >= 1440 rail 264; 1200-1439 rail 248; 1024-1199 rail thu 56; < 1024 rail thành lớp phủ 248. Có hysteresis 24px để không bật tắt liên tục khi kéo cửa sổ |
| S4 | Nút chuyển giao diện | mới | Cũ: "Thử giao diện mới (UI next gen)"; mới: "Về giao diện Orrery cũ" (Settings + Ctrl+K). Đổi ngay, không restart |

## 2. Rail session sống

| ID | Chức năng | Loại | Chi tiết |
|---|---|---|---|
| R1 | Row 2 dòng | đổi | Cao 46-56px. Dòng 1: icon trạng thái + tên + thời gian. Dòng 2: agent, **tên model**, 1 chi tiết ưu tiên (đang chạy: tool + thời gian; chờ: lý do; rảnh: kết quả gần nhất). Gauge context 28x4 bên phải, vàng khi >= 65% |
| R2 | Nhóm Project / Trạng thái | mới | Segmented đầu rail, nhớ `ccm.next.railGroup`. Trạng thái: Lỗi, Đang chạy, Xong chưa xem, Rảnh (Cần bạn ở khối riêng) |
| R3 | Khối "Cần bạn" sticky | mới | Đầu rail, cả 2 chế độ nhóm. Ẩn khi rỗng. Item: icon loại + tên + lý do + thời gian chờ + nút Mở. Nút "Mở kế tiếp" (Ctrl+Shift+A, dùng lại `wsAttention`) |
| R4 | Taxonomy cần bạn | mới | Cần trả lời (icon ?), Cần duyệt quyền (icon khoá), Lỗi (icon x, nhóm riêng), Xong chưa xem (mục riêng, không vào Cần bạn). Sắp: duyệt quyền / câu hỏi, lỗi, xong chưa xem, chờ lâu nhất |
| R5 | Hoãn + hoàn tác | mới | Menu hoãn 5 / 15 / 60 phút trên item Cần bạn; item đã xử lý biến mất kèm toast hoàn tác 5s. Không cho hoãn lỗi quá 15 phút |
| R6 | Không nhảy dưới con trỏ | mới | Khi chuột đang ở rail, đổi trạng thái không đổi thứ tự; áp dụng khi chuột rời rail. Row đã ghim giữ chỗ |
| R7 | Rail thu gọn 56 | đổi | Monogram project 28 + chấm trạng thái từng session + badge số cần bạn / lỗi. Bấm project mở flyout 280 đủ row. Focus bàn phím hiện nhãn ngay |
| R8 | Dữ liệu mới cho rail | mới | `usePaneInfo.modelOf(pane)` từ `agents.mainOf(id).model` + `lib/model.ts::modelLabel`; thiếu thì để trống, không đoán |

## 3. Tab + canvas

| ID | Chức năng | Loại | Chi tiết |
|---|---|---|---|
| T1 | Tab pill | đổi | Strip 36, pill 28, monogram tone project, 1 chip ưu tiên (cần bạn / lỗi / số pane). Phần còn lại ở hover. Tràn thì `+N` |
| T2 | Chip "Chờ trả lời N" | giữ nguyên | v100 `MenuQueueChip`, Ctrl+Alt+Q, vẽ lại kiểu chip mới |
| T3 | Pane card | đổi | Bo 18, header 34 (`PaneHeader` dùng chung cho terminal + 7 viewer), active viền 2px accent, header tint accent 6% |
| T4 | Click menu dưới pane | giữ nguyên | v100 `TermMenuBar`, vẽ lại: nền vàng nhạt, nút có số phím, nút chính đặc accent |
| T5 | Terminal | mới | Mặc định đặc theo palette; Settings "Terminal trong suốt" (sáng 92%, tối 55%, sao Ngân hà xuyên vào) |

## 4. Kéo thả + split (mockup: /drag, /split)

| ID | Chức năng | Loại | Chi tiết |
|---|---|---|---|
| D1 | La bàn 5 ô | mới | Rê pane / file / row lên pane khác: 5 ô 44px (trái, phải, trên, dưới, giữa = đổi chỗ) ở giữa pane đích. Ô đang chọn tô accent, vùng xem trước ghi kết quả ("Chia phải, 50%") |
| D2 | Bố cục dựng sẵn | mới | Nút "Bố cục" ở tab strip: 1 pane, 2 cột, 3 cột, 2 x 2, 1 lớn + cột, Trên + 2 dưới, "Chia đều". Chỉ xếp lại pane có sẵn, không tạo / đóng pane |
| D3 | Thả sang tab khác | mới | Kéo lên tab pill, giữ 600ms (thanh chạy dưới pill) thì chuyển tab, thả tiếp bằng la bàn |
| D4 | Kéo từ Explorer / rail | mới | File từ Explorer: thả = mở viewer ở vị trí đó. Row rail: như kéo pane hiện có |
| D5 | 3 kiểu thả | mới | Chia: nền accent 12% + viền 2px. Đổi chỗ: viền nét đứt quanh 2 pane. Không hợp lệ (pane < 220px, `MIN_PANE`): ô + vùng đỏ, chữ "Quá hẹp: còn Npx, tối thiểu 220px", thả không làm gì |
| D6 | Ghost + pane gốc | đổi | Ghost = pill kính 40px: agent + tên + chip trạng thái. Pane gốc phủ lớp canvas 62% + nhãn "Đang kéo pane này" (không dùng opacity cả pane) |
| D7 | Thanh chia | đổi | Vùng bấm 12px; hover / kéo: vạch accent 2px + grip 3 chấm |
| D8 | Tỉ lệ + hít | mới | Khi kéo hiện "62 / 38". Hít 1/3, 1/2, 2/3 trong 12px; giữ Alt bỏ hít; nhấp đúp về 50/50. Phím mũi tên giữ như cũ |
| D9 | Hoàn tác bố cục | mới | Sau mỗi lần thả / preset / chia đều: toast "Đã chia pane" + nút Hoàn tác (5s). Không gắn Ctrl+Z (terminal, editor đang dùng). Lệnh Ctrl+K "Hoàn tác bố cục" |
| D10 | Gợi ý tách tab | mới | Tab >= 5 pane và có pane < 360px rộng hoặc < 180px cao: banner "Tách N pane sang tab mới?" (Tách / Bỏ qua, nhớ bỏ qua theo tab) |
| D11 | Phím trong menu pane | mới | Menu "..." liệt kê phím có sẵn: chia phải / dưới (`wsSplitRight` / `wsSplitDown`), sang pane (`wsFocus`), đổi chỗ (`wsSwap`), phóng to (`wsZoom`), đóng (`wsClose`). Mới: **Tách sang tab mới** Ctrl+Alt+T (macOS Alt+Cmd+T), đã kiểm không trùng `lib/workspaceKeys.ts` |

## 5. Explorer

| ID | Chức năng | Loại | Chi tiết |
|---|---|---|---|
| E1 | Drawer 288 | đổi | Trượt đè từ phải (Ctrl+Shift+E), lớp mờ canvas 10%, Esc đóng và trả focus |
| E2 | Ghim thành cột | mới | Nút ghim, nhớ `ccm.next.explorerPinned`; chỉ ghim được khi cửa sổ >= 1440 và canvas còn >= 720, không đủ thì tự về drawer + toast giải thích |
| E3 | 4 view | giữ nguyên | Files, Changes (badge số), Search, Checks; TODO vào menu |

## 6. Ctrl+K

| ID | Chức năng | Loại | Chi tiết |
|---|---|---|---|
| K1 | Nhóm Session | mới | Ô rỗng: Cần bạn, Đang chạy, Xong chưa xem, Gần đây, rồi Lệnh. Row 52: icon trạng thái, tên, lý do / tool, project, nhánh, model, thời gian |
| K2 | Tìm theo model / nhánh / repo | mới | Khớp thêm `model`, branch, repo |
| K3 | Phạm vi | đổi | Tab Tất cả, Session (@), File (#), Lệnh (>), Project (:) giữ prefix cũ. Enter mở giữ split; Shift+Enter mở pane mới |

## 7. Settings (dialog kính 1040 x 760)

| ID | Chức năng | Loại | Chi tiết |
|---|---|---|---|
| G1 | Trang "Giao diện" | đổi | Kiểu giao diện (Orrery cũ / Sương kính), Bảng màu (9 thẻ có mini Workspace, bấm chọn), Sáng / tối (Theo hệ thống / Sáng / Tối), Giảm trong suốt, Terminal trong suốt, Chuyển động, Sao Ngân hà, Phông chữ |
| G2 | Trang Workspace, Ổ đĩa | mới | Gom công tắc click menu / menu toast (v100) và Ổ đĩa (v101) thành trang riêng |
| G3 | Tên tránh lặp | đổi | "Giao diện" chỉ còn là tên trang; mục chế độ đổi thành "Sáng / tối" |
| G4 | Khoá lưu | mới | `ccm.uiGeneration`, `ccm.nextTheme`, `ccm.next.railGroup`, `ccm.next.explorerPinned`, `ccm.next.reduceGlass`, `ccm.next.clearTerm`, `ccm.next.motion` |

## 8. Motion (`07-motion.md` mục B)

| ID | Chức năng | Loại | Chi tiết |
|---|---|---|---|
| M1 | Setting 3 mức | mới | Đầy đủ / Giảm / Tắt, root `data-motion`; mặc định theo `prefers-reduced-motion` |
| M2 | Token | đổi | instant 80, fast 150, base 220, slow 320 ms; ease standard / enter / exit |
| M3 | Tín hiệu 1 lần | mới | Session sang cần bạn / lỗi: viền sáng 450ms 1 lần, không nhấp nháy mãi |
| M4 | Tan bụi 700ms | đổi | Từ 1000ms |
| M5 | Bỏ sao băng trong panel | đổi | Bỏ `ccm-meteor-pane` ở UI mới, giữ sao băng nền |
| M6 | Dừng khi ẩn | mới | Cửa sổ ẩn / thu nhỏ: dừng mọi animation lặp |
| M7 | Đổi layout bằng transform | đổi | Tab pill, segmented thumb, rail fold, drop zone bỏ animate `width` / `left` / `grid-template-rows` |

## 9. Design system (WP1)

Primitive bắt buộc, thay chỗ lặp: `GlassDialog` (480 / 640 / 880, focus trap chung), `GlassMenu` (220-280, row 32), `GlassTooltip` (450ms, focus hiện ngay), `SegmentedControl` duy nhất, chip h20 duy nhất, `PaneHeader`, 1 context gauge.
Token màu / chữ / cỡ: `ui-inventory/06-codex-design-proposals.md`, có 2 chỉnh: working = xanh dương (`#2563A8` / `#8AB4FF`) tách khỏi vàng của cần bạn; `text-2xs` = 11px.

## 10. Còn mở

- **Q1 Kính hay flat**: mockup có cả 2 (ô style). Chủ nhân chưa chọn. Phương án con đề xuất: kính mặc định, flat = lựa chọn "Giảm trong suốt" mở rộng (bỏ blur + bo góc nhỏ).
