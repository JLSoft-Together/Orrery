## WP-v87-1-idle — DONE

- Đã đổi:
  - `apps/web/src/composables/usePageActive.ts` (mới) — `pageActive` ref theo `document.visibilityState` + `useActiveInterval(fn, ms, {catchUp, immediate})`: dừng khi ẩn, chạy bù 1 lần khi hiện lại.
  - `apps/web/src/composables/usePageActive.test.ts` (mới) — 4 test fake timers + mock visibility.
  - `apps/web/src/components/FlipClock.vue` — chuỗi setTimeout 1 s dừng khi ẩn, tick ngay khi hiện (chỉ `<script>`).
  - `apps/web/src/components/history/HistoryView.vue` — poll `loadTimeline` 30 s qua `useActiveInterval`; bỏ `onBeforeUnmount` import thừa.
  - `apps/web/src/stores/toolSpans.ts` — prune 60 s qua `useActiveInterval`.
  - `apps/desktop/src/main.ts`, `preload.ts`: KHÔNG đổi. Audit: `webPreferences` không có `backgroundThrottling:false`, không `paintWhenInitiallyHidden`, không `setBackgroundThrottling`. `powerSaveBlocker('prevent-app-suspension')` (main.ts:1099) chỉ cho tính năng giữ máy thức, không ép renderer. Minimize / hide tray đều làm `document.visibilityState = hidden` trong Chromium nên không cần kênh IPC trạng thái cửa sổ.

- Bảng timer (apps/web/src), trước → sau khi ẩn:
  | file:line | chu kỳ | trước | sau |
  |---|---|---|---|
  | lib/format.ts:18 clock `now` | 1 s | đã dừng khi ẩn (v86) | giữ nguyên |
  | components/FlipClock.vue tick | 1 s | chạy | dừng, tick bù khi hiện |
  | components/history/HistoryView.vue (loadTimeline) | 30 s | chạy | dừng, load bù khi hiện |
  | stores/toolSpans.ts prune | 60 s | chạy | dừng, prune bù khi hiện |
  | components/TelegramSettings.vue:643 | 30 s | đã gate visibility | giữ nguyên |
  | components/DiagnosticsDialog.vue:138 | 5 s | chỉ khi dialog mở | giữ (dialog đang mở thì chủ nhân đang xem) |
  | components/devices/DeviceCaptureDialog.vue:293 | tick | chỉ khi dialog mở | giữ |
  | composables/useCompact.ts:107 sweep | 15 s | chỉ khi có compact đang chờ | giữ: là deadline chờ bước compact, không được dừng khi ẩn |
  | composables/usePaneAttachments.ts:164 | pollMs | chỉ trong waitFor ngắn | giữ |
  | stores/peers.ts:201 | theo mốc hết hạn note | one-shot tới mốc, chỉ khi có note | giữ |
  | composables/useWorkspaceLinks.ts:29 | one-shot, retry 10 s khi host không trả | event-driven | giữ |
  | stores/resumeSchedule.ts:56 | one-shot tới giờ hẹn | cần chạy khi ẩn (resume theo lịch) | giữ |
  | office/sound.ts, lib/sound.ts, lib/highlight.ts | one-shot đóng khi rảnh | đã làm v86 | giữ |
  | components/office/OfficeView.vue | feed gate `document.hidden` | đã gate | giữ |
  | AgentMap.vue, ui.ts, toasts.ts, … setTimeout | one-shot UI | không lặp | giữ |
  Toast / attention / Telegram: không đổi, không phụ thuộc các timer trên; `useWorkspaceLinks` attention logic vẫn chạy khi ẩn (watcher, không timer). Collector đã có `client.presence {attentive}` (connection.ts:340) nên mục 3 coi như có sẵn.

- Tiêu chí: 3/3 đạt (bảng timer; test composable; web test + typecheck pass; desktop không đổi nên không chạy test desktop).
- Audit: xong (trên). Review: edge case minimize→restore dữ liệu cập nhật ngay nhờ catch-up; tray ẩn = visibility hidden; compact window dùng cùng cơ chế; toast / attention / Telegram / collector không phụ thuộc renderer timer.
- Test: `npm test -w @ccm/web -- --maxWorkers=4` → 1827 pass / 0 fail (214 files, gồm 4 test mới); `npm run typecheck -w @ccm/web` → sạch.
- Lưu ý: `apps/web/src/stores/connection.ts:355-361` có thụt lề lệch (của người khác), không đụng.
- Đề xuất chưa làm: (1) collector gộp event thưa 1 s theo `client.presence.attentive=false` (protocol đã có cờ, chỉ cần phía collector); (2) virtualize list: activity / history sessions (HistoryView phân trang 20, ok) / git log (LogTable) / explorer chưa kiểm số item > 200 vì chưa tìm thấy thư viện virtualize trong repo; (3) gate `FilePane`/`ChangesPane` refresh khi ẩn (event-driven, rủi ro thấp).
- Commit message đề xuất:
  Pause 1 s clock, history poll and tool-span prune while the window is hidden; add usePageActive composable
