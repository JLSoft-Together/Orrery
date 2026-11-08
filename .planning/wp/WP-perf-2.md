## WP-perf-2: Renderer CPU trong Workspace — việc theo từng chunk output
- Mục tiêu: bớt CPU main thread renderer khi các pane terminal stream output, theo PLAN-v86 Gói 1.2–1.3 + clock. Không đổi hành vi user thấy.
- Phạm vi file: `apps/web/src/components/workspace/TermPane.vue`, `apps/web/src/lib/devServer.ts`, `apps/web/src/composables/usePaneAttachments.ts`, `apps/web/src/stores/attachments.ts`, `apps/web/src/stores/workspace.ts` (chỉ watcher persist ~558-600), test tương ứng.
  Cấm đụng: `App.vue`, `lib/highlight.ts`, `lib/sound.ts`, `office/`, `AgentMap.vue`, locale files, `apps/desktop`, `apps/collector`.
- Song song với: WP-perf-1, WP-perf-3.
- Model: sonnet.
- Việc:
  1. `feedDev` (`TermPane.vue:113-119`, `devServer.ts:48-66`): chỉ chạy cho pane shell (không chạy cho pane agent Claude/Codex/DeepSeek). Kiểm chỗ dùng kết quả dev server để chắc không mất tính năng (dev server URL detect).
  2. Attachment scan (`usePaneAttachments.ts:103-124`, `readScreen` `TermPane.vue:587-603`): đổi sang debounce chờ output lặng (trailing, vd 150–250 ms, có max chờ ~1 s khi output liên tục), chỉ quét khi pane đang hiện.
  3. `attachments.ts:71,155` `reconcile`: không tạo `byPane` mới / không `set()` khi danh sách không đổi (so nông theo id/thứ tự).
  4. `workspace.ts:558-600` watcher `deep: true` trên `panes`: debounce 300–500 ms trước `JSON.stringify` + ghi `sessionStorage`/`localStorage`; flush ngay khi `beforeunload`/`pagehide` để không mất layout khi đóng app.
  5. Clock 1 s (`TermPane.vue:~286`) không chạy khi pane ẩn.
- Tiêu chí nghiệm thu:
  - [ ] Test: reconcile cùng danh sách → không đổi reference `byPane`.
  - [ ] Test: persist debounce gộp nhiều patch thành 1 lần ghi; flush khi pagehide.
  - [ ] Test (nếu khả thi) scan debounce: 20 lần onWriteParsed liên tiếp → 1–2 lần scan.
  - [ ] Dev server detect vẫn chạy cho pane shell (test có sẵn trong `devServer` pass).
  - [ ] `npm test -w @ccm/web` + `npm run typecheck -w @ccm/web` pass.
- Lệnh test: `npm test -w @ccm/web -- <file>` cho từng file, cuối cùng `npm test -w @ccm/web`; `npm run typecheck -w @ccm/web`.
- Nhìn thấy được: không (hành vi giữ nguyên).
- Báo cáo: `.planning/wp/WP-perf-2-report.md`
- 2026-11-09 step1: edit TermPane (feedDev shell-only, clock hidden), usePaneAttachments (trailing debounce), attachments set no-op, workspace persist debounce

## Nhật ký
- step1: sửa TermPane/usePaneAttachments/attachments/workspace persist debounce
- step2: viết test (attachments, usePaneAttachments scan, workspacePersist); đang chạy vitest
- step3: sửa test cũ (workspace/workspaceRestore/fileEditor) thêm persistNow; full vitest 1802/1803 -> chạy lại fileEditor
