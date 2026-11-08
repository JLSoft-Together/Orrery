## WP-perf-4: Pane terminal ẩn — gom output, WebGL LRU, scrollback
- Mục tiêu: giảm CPU renderer + RAM khi có nhiều pane (nhiều tab) đang stream. PLAN-v86 Gói 1.4 + 4.4.
- Hiện trạng: pane của mọi tab mount sẵn bằng `v-show` (`WorkspaceView.vue:~1629-1636`), pane ẩn vẫn `term.write` + parse toàn bộ output; WebGL dispose khi ẩn (`TermPane.vue:~525-539`) → chuyển tab dựng lại atlas, giật một nhịp; `scrollback: 5000` (`TermPane.vue:~888`, `DropTerminal.vue:~198`). Output tới qua `ptyClient.ts:~94` → `TermPane.vue:~797-801` (`term.write` + `feedDev`). PTY host giữ ring 256 KB replay (`apps/desktop/src/pty/host.ts:~43,188,284`).
- Việc:
  1. Pane ẩn: không `term.write` từng chunk; gom vào buffer (max ~2 MB/pane, vượt thì bỏ phần đầu và đánh dấu cần replay — dùng cơ chế replay/ring sẵn có của PTY host nếu có API, nếu không thì giữ phần đuôi), ghi một lần khi pane hiện lại (hoặc theo đợt lớn khi idle). Đảm bảo không mất byte/thứ tự, không phá escape sequence giữa chừng; status/attention/title detection hiện đang dựa vào output phải vẫn chạy (kiểm chỗ nào đọc buffer xterm cho pane ẩn — nếu có, giữ đúng).
  2. WebGL: giữ addon WebGL cho tối đa 4 pane dùng gần nhất (LRU) thay vì dispose ngay khi ẩn; pane thứ 5 ẩn lâu nhất thì dispose. Context lost vẫn fallback như hiện tại.
  3. Scrollback 5000 → 3000 cho TermPane và DropTerminal (hằng số chung nếu có).
- Phạm vi file: `components/workspace/TermPane.vue`, `components/workspace/WorkspaceView.vue` (chỉ nếu cần truyền visible), `components/workspace/DropTerminal.vue`, `lib/ptyClient.ts`, file lib mới cho buffer/LRU (vd `lib/termBuffer.ts`, `lib/webglLru.ts`) + test.
  Cấm đụng: `App.vue`, `AgentMap.vue`, `stores/ui.ts`, `stores/office.ts`, settings, locale (WP-perf-6), `apps/desktop/src/main.ts` (WP-perf-5), `apps/collector`.
- Model: opus (xterm write ordering + lifecycle, dễ mất output).
- Tiêu chí nghiệm thu:
  - [ ] Unit test buffer: nhiều chunk khi ẩn → hiện lại ghi đúng thứ tự, đủ byte; vượt max → giữ đuôi + cờ truncated; unmount giữa chừng không leak.
  - [ ] Unit test LRU: 6 pane lần lượt hiện → chỉ 4 giữ WebGL; pane hiện lại được đưa lên đầu.
  - [ ] Scrollback 3000 ở cả 2 chỗ.
  - [ ] `npm test -w @ccm/web` (full, `--maxWorkers=6`) + `npm run typecheck -w @ccm/web` pass (WP-perf-6 có thể đang sửa file web khác cùng lúc; lỗi ở file của nó thì ghi lại, không sửa).
- Nhìn thấy được: có (chuyển tab với pane đang stream) → CHƯA NHÌN, không được chạy app.
- Báo cáo: `.planning/wp/WP-perf-4-report.md`

## Nhật ký
- 2026-11-09 audit: TermPane ghi mọi chunk vào xterm kể cả pane ẩn; phụ thuộc output khi ẩn: OSC 7/9 (cwd + pendingCommand), OSC 133 marks, snapshot Mission Control / Telegram tail, reply DSR/DA cho TUI.
- 2026-11-09 code: lib/termBuffer.ts (HiddenOutput: gom khi ẩn, flush khi hiện / 1s / khi program hỏi reply / trước snapshot, max 2 MB giữ đuôi), lib/webglLru.ts (giữ 4), TermPane + DropTerminal scrollback 3000.
- 2026-11-09 test: termBuffer.test.ts + webglLru.test.ts 17/17 pass.
- 2026-11-09 verify: full web test 1823/1823 pass, typecheck pass; report WP-perf-4-report.md.
- 2026-11-09 vòng 2: carry giữ qua flush, bỏ truncate (vượt 2 MB thì flush), refit chờ backlog parse xong, snapshot ghi rõ là sync; full test 1823/1823, typecheck pass.
