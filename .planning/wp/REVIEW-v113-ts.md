# Review v113 TS / Vue (reviewer sonnet, chỉ đọc, 27/11)

0 CAO, 3 VỪA, 5 THẤP.

## VỪA
1. NGHI. `features/remote/components/connect/useConnectWizard.ts` watch `remote.devices/pairing/pairEnded`: ticket hết TTL, `status.pairing` về false mà `pairEnded` không set (shared chỉ có `'tries'|'denied'`). Status về trước tick `now` → wizard nhảy `pairConfirm`, state này không có timeout → kẹt. Sửa: timeout cho `pairConfirm` hoặc `pairing` về false mà không có device mới → `pairExpired`.
2. CHẮC. `useConnectWizard.ts` `doTest` / `onPaired`: `paired.value` sống suốt vòng đời `SettingsRemote`, `start()` không reset. "Kiểm lại" từ hero / idle test nhầm máy cũ (kể cả đã revoke) → `unknownCheck`. Sửa: reset `paired` khi `start`, `doTest` ưu tiên máy đang connected.
3. NGHI. `features/run/stores/multiRun.ts` `settle()`: session bị xoá khỏi `run.sessions` sau khi đã có, hoặc kẹt state chưa running chưa ended → không timer, `runMany` treo, Huỷ chỉ có tác dụng khi `run.stop` đưa về `stopped`. Sửa: deadline cho cả nhánh session đã có, session biến mất → `session-missing`.

## THẤP
1. `features/run/lib/apkOffer.ts` `maybeAutoOffer`: mở lại app, session đang chạy có `apk` mà `prev` undefined → auto offer lại; set `offered` không dọn.
2. `features/devices/composables/useSettingsDevices.ts`: `unmerge` không xử lý link `hash`; `loadPkg` không bắt reject (`void syncPkg()` unhandled); load muộn ghi đè `pkg` đã dọn của máy offline.
3. `features/devices/lib/mergeDevices.ts` `suggestions`: không đánh dấu adb đã gợi ý → 2 remote cùng model cùng gợi ý 1 adb.
4. `apps/pwa/src/lib/deepLink.ts` + `ChatView.vue`: `pushMenu` ở lại bộ nhớ khi pane chưa online / stale, về sau tự gửi đáp án khi pane có menu cùng sig → cần TTL (vd 2 phút) và chỉ gửi trong phiên mở từ thông báo.
5. `connectFlow.ts` / `remoteInstall.ts::parseTicket`: `path.slice(0, -'orrery.apk'.length)` giả định đuôi `orrery.apk`; `features/git/lib/checklist.ts::runCheck`: `launch` throw thì state `running` vẫn lưu.

## Đã soát, không thấy lỗi
multiRun restore trong `finally` (cả huỷ); service worker chỉ gửi menu khi bấm action, sig / index / machine / pane qua regex; `v-html` chỉ SVG QR của thư viện `qrcode`; revoke / undo đúng id; `stageApk` kiểm chữ ký đúng; `extraLan` validate ở Rust; listener / timer / abort gỡ trong `onBeforeUnmount`.
