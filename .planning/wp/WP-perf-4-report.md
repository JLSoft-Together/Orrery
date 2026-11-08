## WP-perf-4 — DONE

- Đã đổi:
  - `apps/web/src/lib/termBuffer.ts` (mới): `HiddenOutput` — pane hiện thì ghi thẳng; pane ẩn thì gom chunk, ghi một lần khi: pane hiện lại (`setVisible(true, done)`), timer 1s (`HIDDEN_FLUSH_MS`), program hỏi terminal reply (DSR `CSI 6n`, DA `CSI c`/`>c`, DECRQM `$p`, DECRQSS/XTGETTCAP, OSC 10/11 `?`, `CSI ?u`, XTWINOPS `CSI n t`, XTVERSION; bắt cả khi bị cắt giữa 2 chunk), hoặc trước khi đọc snapshot. Max 2 MB/pane (`HIDDEN_MAX_BYTES`): vượt thì bỏ phần đầu, cờ `truncated`, lúc ghi cắt đuôi ở ESC / sau LF gần nhất (≤4 KB) để không bắt đầu giữa sequence / giữa ký tự UTF-8. `TERM_SCROLLBACK = 3000`.
  - `apps/web/src/lib/webglLru.ts` (mới): `WebglLru` / `webglPool` giữ WebGL cho 4 pane gần nhất; chỉ evict pane đang ẩn (tab split > 4 pane vẫn giữ hết pane đang hiện, như cũ).
  - `TermPane.vue`: `onOutput` → `output.push`; `output.setVisible` trong onMounted và watch `visible` (hiện lại: ghi dồn → nextTick → focus → chờ xterm parse xong → `refit`, để byte sinh theo size cũ không bị parse sau resize); `output.clear()` trong `connect`/`restart`/`reconnect` (trước `term.reset`); snapshot reader `output.flush()` trước khi đọc; `setRenderer` dùng `webglPool.show/hide`, `dropRenderer()` cho context lost + unmount; `output.dispose()` khi unmount; scrollback `TERM_SCROLLBACK`.
  - `DropTerminal.vue`: scrollback `TERM_SCROLLBACK` (renderer giữ nguyên, nó chỉ là 1 terminal).
  - Test mới: `lib/termBuffer.test.ts` (13), `lib/webglLru.test.ts` (4).
  - Giữ nguyên thay đổi đợt trước (`feedDev` chỉ shell, `activityText` khi ẩn, `usePaneAttachments`). Line ending CRLF giữ nguyên ở file sửa và file mới. Không đụng `ptyClient.ts`, `WorkspaceView.vue`.
- Tiêu chí: 4/4 đạt.
- Audit: thứ tự byte giữ nhờ 1 hàng đợi duy nhất (pending luôn ghi trước chunk live); xterm giữ trạng thái parser qua các lần write nên sequence bị cắt giữa chunk không vỡ. Chỉ phần bị cắt khi vượt 2 MB là mất (giống ring 256 KB của PTY host vốn chấp nhận replay giữa sequence). Không dùng replay của host khi truncated: ring host 256 KB < đuôi 2 MB giữ sẵn, reattach không thêm gì.
- Review edge case:
  - Escape bị cắt giữa chunk: an toàn (cùng stream). Test có case `\x1b[3` | `1mred`.
  - Alt-screen Claude/Codex khi replay: replay khi pane ẩn được gom và ghi đúng thứ tự sau `ready`; DSR/DA trong lúc ẩn (Codex/crossterm hỏi vị trí con trỏ, timeout 2s) được ghi ngay nên reply không trễ.
  - Resize lúc ẩn: `refit` vốn bỏ qua pane ẩn; khi hiện thì refit sau khi parse xong phần dồn.
  - Pane đóng lúc ẩn: unmount → `output.dispose()` huỷ timer, bỏ buffer (test dispose); `dropRenderer` gỡ khỏi pool.
  - OSC 7/9 (cwd, gửi `pendingCommand`), OSC 8, OSC 52, OSC 133 marks khi ẩn: chạy trễ tối đa ~1s (timer flush), vẫn trước grace 4s của `pendingCommand`.
  - Find/search, `readScreen` attachment, `scanErrors`: chỉ dùng ở pane đang hiện; khi hiện lại `onWriteParsed` → `att.schedule` sau khi phần dồn parse xong.
  - Snapshot `lib/paneSnapshots.ts` (Mission Control, Telegram `tail`): reader gọi `output.flush()`; vì xterm parse bất đồng bộ nên lần đọc đó có thể trễ tối đa ~1s nội dung, lần poll sau đã đủ.
  - Context lost khi pane ẩn còn giữ WebGL: `dropRenderer` → về DOM renderer như cũ, hiện lại tạo WebGL mới.
- Test: `npm test -w @ccm/web -- src/lib/termBuffer.test.ts src/lib/webglLru.test.ts` → 17 pass / 0 fail; `npm test -w @ccm/web -- --maxWorkers=6` → 1823 pass / 0 fail (213 file); `npm run typecheck -w @ccm/web` → pass.
- CHƯA KIỂM: tích hợp trong TermPane (xterm thật + WebGL thật) — không có test component cho TermPane, không được chạy app.
- Nhìn: CHƯA NHÌN: không được chạy app (chuyển tab với pane đang stream, hiện lại pane ẩn lâu, 5+ tab giữ WebGL).
- Ngoài phạm vi:
  - `.code_index/web.md` / `_meta.md` chưa cập nhật (tránh ghi đè song song với WP khác) — main thêm: `lib/termBuffer.ts` `HiddenOutput` (gom output pane ẩn, flush 1s / khi hiện / khi hỏi reply / trước snapshot, max 2 MB), `lib/webglLru.ts` `webglPool` (4 WebGL), `TERM_SCROLLBACK` 3000.
  - Pane ẩn vẫn parse theo đợt 1s (không hoãn hẳn tới lúc hiện) vì OSC 7 / `pendingCommand` / marks / snapshot Mission Control cần nội dung pane ẩn. Muốn hoãn hẳn thì cần chủ nhân chốt chấp nhận snapshot Mission Control cũ.
- Commit message đề xuất:
  `perf(workspace): batch hidden pane output, keep WebGL for 4 recent panes, scrollback 3000`

## Vòng 2 (sửa theo nghiệm thu) — DONE
Mục Vòng 2 này thay cho các dòng ở trên nói về truncate 2 MB / `safeStart` / snapshot mới ngay.
- `lib/termBuffer.ts`:
  1. `carry` (đuôi tối đa 31 ký tự từ ESC cuối) giữ qua các lần flush, chỉ xoá trong `clear()`; cập nhật cả khi pane hiện (`keepTail`, chỉ decode 31 byte cuối, nên đường ghi thẳng gần như không tốn thêm). Query đã khớp được cắt khỏi `carry` (`REPLY_ALL`) để không flush lặp lại.
  2. Bỏ truncate + `safeStart` + `truncated`: buffer vượt max 2 MB thì flush ngay toàn bộ vào xterm, không mất byte nào (giữ mode sequence alt screen, bracketed paste, mouse, kitty, `?25l`, không bắt đầu giữa OSC/DCS).
- `TermPane.vue`:
  3. Snapshot: `snapshotOf` / reader là sync (Mission Control, Telegram `tail` đọc string ngay), không chờ được write callback. Giữ `output.flush()` trong reader cho đơn giản: lần đọc đó vẫn là nội dung cũ (xterm parse bất đồng bộ, trễ tối đa ~1s timer + thời gian parse), lần poll sau mới có phần vừa ghi. Không claim snapshot mới ngay.
  4. `showBacklog()` + biến `backlog`: khi hiện lại, `refit()` (gồm từ ResizeObserver, `applyCodeFont`) bỏ qua tới khi xterm parse xong phần dồn; watcher refit sau đó. Hiện/ẩn liên tiếp thì backlog cũ được resolve, backlog mới chặn tiếp. Unmount gọi `backlog?.()` nên promise không treo (refit sau đó tự thoát vì `host` đã null).
- Test `termBuffer.test.ts` (13): bỏ 2 test truncate + 2 test `safeStart`; thêm: vượt max ghi đủ byte kèm `\x1b[?1049h`, query cắt qua lần flush timer (`…\x1b[6` | `n`), query cắt qua lúc chuyển sang ẩn, query đã trả lời không gây flush lặp.
- Test: `npm test -w @ccm/web -- --maxWorkers=6` → 1823 pass / 0 fail (213 file); `npm run typecheck -w @ccm/web` → pass. CRLF giữ ở cả 3 file sửa.
- CHƯA KIỂM: tích hợp TermPane với xterm thật (không có test component). CHƯA NHÌN: không được chạy app.
- Commit message đề xuất (không đổi): `perf(workspace): batch hidden pane output, keep WebGL for 4 recent panes, scrollback 3000`
