## WP-114a — DONE (E2E và nhìn bằng mắt: CHƯA KIỂM)

### Đã đổi
Mới:
- `apps/web/src/features/next/lib/railPeek.ts` (236 dòng, thuần): controller `createPeekCtl` (1 timer duy nhất, dwell 400 ms, grace 120 ms đóng khi rời, chuyển hàng khi đang mở thì hiện ngay), `peekContent` (nội dung theo loại pane), `maskQuestion` (maskText + `maskSecrets` của `@ccm/shared`, gộp khoảng trắng, max 120 ký tự), `placePeek` (bên phải, lật sang trái nếu hết chỗ, kẹp dọc), `openSpanStart`, `shortCwd`, `isPeekKey`.
- `apps/web/src/features/next/lib/useRailPeek.ts` (128 dòng): adapter DOM. Delegate `pointerover` / `pointerout` trên `[data-live-rail]` (đọc `data-row-key`, `relatedTarget`), `pointerdown` capture (bắt đầu kéo thì đóng), `scroll` capture, `focusout` (peek mở bằng phím), `resize` chỉ gắn khi đang mở, `useDismiss` (Esc, pointerdown ngoài, blur cửa sổ, tab ẩn). Bỏ `title` của nút hàng khi mở (gỡ attribute trực tiếp, trả lại khi đóng) và gắn `aria-describedby`. Export `peekHub` để rail thu gọn (flyout Teleport ra body) dùng chung 1 controller.
- `apps/web/src/features/next/components/rail/RailPeek.vue` (163 dòng): 1 instance, Teleport body, `v-if` nên chưa rê thì không có popover; fade `ccm-next-fade` (opacity, 220/160 ms theo token, reduced-motion đã có sẵn trong token). Nội dung: tên pane, chip trạng thái (icon + chữ + token màu), tool + `đang chạy {t}` từ span mở mới nhất, câu hỏi menu đã mask (fallback nhãn chờ), thời gian ở trạng thái, nút "Mở pane" (không có nút trả lời). Pane không phải agent: tên + loại + cwd rút gọn. Agent không dữ liệu: "Chưa có hoạt động". Đóng khi đổi tab / view (`watch` chỉ có dependency khi đang mở) và khi hàng biến mất.
- Test: `lib/railPeek.test.ts` (29 test), `components/rail/RailPeek.test.ts` (11 test).
- E2E viết sẵn: `apps/shell/e2e/rail-peek.cjs` (`node --check` ok, CHƯA CHẠY).

Sửa nối:
- `LiveRail.vue` +2 dòng (import + `<RailPeek :row-of="rowOf" @open="activateRow" />`).
- `RailCollapsed.vue` +3 dòng (import + `@pointerover` / `@pointerout` trên flyout).
- `RailRowItem.vue` +12 dòng (hàm `onKey`: phím `i` gọi `peekHub.toggle`, còn lại emit `key` như cũ; bỏ qua khi đang rename). Hơn 1-5 dòng vì cần hàm xử lý phím; file này 185 dòng nên không vi phạm trần dòng.
- `features/next/i18n/v106-rail.ts`: nhóm `v114peek` (en + vi: label, empty, tool, question). Chuỗi còn lại tái dùng `v105u1.*` ("Mở pane", trạng thái, đơn vị thời gian).

### Tiêu chí
- Rê ≥ 400 ms mới mở, cả rail đầy đủ lẫn flyout của rail thu gọn, cùng 1 instance: ĐẠT (vitest).
- Nội dung theo D2 (tool + thời gian + câu hỏi, không dòng chữ terminal): ĐẠT.
- Đóng: rời hàng + popover, Esc, cuộn rail, bắt đầu kéo, resize, đổi tab: ĐẠT (vitest). Rời rail qua đường chéo có grace 120 ms để chuyển sang popover.
- Bỏ `title` native khi mở, trả lại khi đóng: ĐẠT. Phím `i` + `aria-describedby`: ĐẠT.
- Mask câu hỏi: ĐẠT (token `token=...`, `sk-...`, thư mục user Windows).
- Không subscribe / timer khi chưa rê: ĐẠT trong vitest (số timer không đổi sau mount, và khi `buttons` đang nhấn); store chỉ bị đọc trong computed khi popover mở.
- Hiệu năng (0 long task > 50 ms khi rê 30 hàng, `WorkspaceView` không render lại, popover ≤ 1): CHƯA KIỂM: cần chạy E2E CDP; `rail-peek.cjs` đã có phép đo long task + đếm popover (8 vòng qua mọi hàng), chưa đo `WorkspaceView` render.

### Audit
- Diff `git diff --stat -- apps/web/src/features/next`: file của gói là LiveRail, RailCollapsed, RailRowItem, v106-rail (+ file mới). `railSnooze.ts` có diff nhưng là của WP-114c, không đụng.
- Không comment trong code mới. Không em dash / dấu chấm giữa trong chuỗi mới. Không blur mới (dùng lại `glass-pop` có sẵn của next).
- File mới đều < 400 dòng.

### Review
- Edge: pointer rời qua khe 6 px giữa hàng và popover: grace 120 ms + `overCard` hủy timer.
- Edge: `buttons !== 0` (đang kéo / chọn) thì bỏ qua `pointerover`, nên sau khi bắt đầu kéo không mở lại.
- Edge: peek mở bằng phím thì mất focus hàng sẽ đóng; peek mở bằng chuột thì focus không ảnh hưởng.
- Rủi ro nhỏ: nếu `openTitle` của hàng đổi nội dung trong lúc peek mở, Vue có thể gắn lại `title` native (tooltip hiện cùng popover). Chấp nhận, hiếm (cần đổi trạng thái đúng lúc rê).
- Rail thu gọn: chấm trạng thái (`data-rail-dot`) 6 px không có `data-row-key` nên không peek; peek hoạt động ở hàng trong flyout. Nếu muốn peek ngay từ chấm cần thêm `data-row-key` vào chấm (sửa `RailCollapsed.vue`, quyết định của main).
- Flyout thu gọn tự cuộn (`overflow-y-auto`) không đóng peek theo scroll (sự kiện scroll không nổi lên nav); đã có đóng bằng pointerdown / Esc.
- Phím `i` cố định (V6 của review đề xuất `i` hoặc `Alt+↓`); không xung đột `onRowKey` (chỉ xử lý mũi tên / Home / End / F2).

### Test
- `npm test -w @ccm/web -- src/features/next --hookTimeout=60000` → 24 file, 203 test pass.
- `npm test -w @ccm/web -- src/i18n src/importRules.test.ts` → importRules pass; `keys.snapshot.test.ts` 2 fail đúng dự kiến (key mới `v114peek.*`, snapshot do main cập nhật; không chạy `-u`). Snapshot cũng sẽ có key của WP-114f.
- `npm run typecheck` → 1 lỗi ngoài phạm vi: `src/features/prompts/composables/useInsertPrompt.ts(35,51): TS2322` (WP-114f). Không có lỗi ở file của WP-114a.
- E2E `rail-peek.cjs`: CHƯA KIỂM: brief cấm chạy E2E.

### Nhìn
CHƯA NHÌN: chưa chạy app, chưa chụp màn hình. Ảnh dự kiến do E2E ghi ở `docs/qa/v114/WP-114a/` (01-peek-shell, 02-peek-collapsed).

### Hot file cần thêm
- `apps/web/src/i18n/__snapshots__/keys*` : cập nhật snapshot cho nhóm `v114peek` (4 key en + 4 key vi).
- `.code_index/` + `.code_index/_meta.md`: thêm `RailPeek.vue`, `railPeek.ts`, `useRailPeek.ts` (ngoài phạm vi gói, main làm), rồi `node scripts/index-check.mjs`.
- `apps/web/src/features/next/README.md`: nhắc i18n `v106-rail.ts` giờ có thêm `v114peek` (tùy chọn).

### Ngoài phạm vi
- Dòng chữ terminal (D2 đã loại). Nút trả lời trong popover. Peek từ chấm rail thu gọn. Touch (bỏ qua `pointerType === 'touch'`, desktop only).

### Commit message đề xuất
feat(next): add hover peek popover for live rail rows

Dwell-based quick look (tool, running time, masked pending question) for rail rows in both the full and collapsed rail, with Esc/scroll/drag/tab close rules, key `i` toggle and a single delegated listener set.
