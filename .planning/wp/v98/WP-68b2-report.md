# WP-68b2 report: hạn mức sang điện thoại

## Đường gửi đã chọn: không đổi đường gửi, dùng đường có sẵn
Phát hiện: điện thoại đã nhận số hạn mức thật.
- `MonitorSnapshot.limits` (Claude Code) và `codexLimits` (Codex) nằm sẵn trong `data` của event `snapshot`; đổi giá trị qua event `limits.updated` / `codexLimits.updated`.
- Gateway (`apps/shell/src/remote/dashboard.rs`) chỉ bỏ `DESKTOP_ONLY` (không có 2 event này); `fit.rs` chỉ cắt `activity`, sub-agent, chuỗi dài, không đụng `limits`.
- Footer next (`stores/extras` ← `snapshot.limits` / `codexLimits`) dùng đúng nguồn này, nên điện thoại và desktop cùng một số.
Lý do không thêm `quota[]` vào summary (`ws.snapshot`) hay dashboard: sẽ gửi trùng một số thật lần thứ hai, thêm hash / debounce / patch và nguy cơ lệch số, mà không có lợi. Không gửi dày hơn nhịp hiện có (không đổi gì ở desktop). Giới hạn 1 MiB / `fit.rs` giữ nguyên.

## File đổi
- `packages/shared/src/remote.ts`: kiểu `RemoteQuota` (`agent`, `window`, `pct`, `resetAt`, `stale`).
- `apps/pwa/src/lib/quotaModel.ts` (mới): `quotaOf(snapshot, now)`, `quotaByAgent`, `formatPct`, `formatReset` / `formatResetAt` (dùng `duration.*` của WP-60 qua `formatElapsed`).
- `apps/pwa/src/lib/quotaModel.test.ts` (mới).
- `apps/pwa/src/i18n/en.ts`, `vi.ts`: nhóm `quota` (label, window5h, windowWeek, resetsIn, stale, used).
- `apps/shell/src/remote/unit_tests.rs`: test `plan_limits_reach_the_phone_in_the_snapshot_and_in_events` (khoá hợp đồng: limits đi qua gateway).
- `docs/remote-integration.md` (§4.2, mục "Hạn mức"), `.code_index/_meta.md`, `.code_index/web-pwa.md`.

## Quy tắc số thật
Thiếu `limits`, window thiếu, `usedPct` không phải số hữu hạn: không có phần tử (không hiện 0%). `pct` làm tròn và kẹp 0-100; `stale` khi `updatedAt` cũ hơn 30 phút (như desktop). Store theo máy: màn gọi `quotaOf(view.snapshot, now)` cho từng `MachineView` (không cần store thêm vì `MachineData` đã giữ snapshot theo máy và áp `limits.updated`).

## Android
Không có field mới nên không phải parse gì thêm. `monitor_reducer.dart` giữ `limits` thô, không UI đọc, không lỗi. Không đụng Dart, không chạy `apps/mobile/scripts/check.mjs`.

## Test (lệnh chạy thật)
- `npm run typecheck -w @ccm/pwa`: sạch.
- `npm test -w @ccm/pwa`: 23 file, 217 test pass (gồm `quotaModel.test.ts`).
- `cargo test -j 4 --bin orrery-shell remote::` (apps/shell): xem kết quả cuối ở tin nhắn tóm tắt.
- Không chạy vitest web (không đổi `apps/web`).

## Việc dở / lưu ý cho WP-68b3
- Chưa có UI: WP-68b3 vẽ thẻ máy + chân rail iPad bằng `quotaByAgent(quotaOf(view.snapshot, now))`, tên agent lấy từ i18n sẵn có (`claude` / `codex`), nhãn cửa sổ `quota.window5h` / `quota.windowWeek`, `quota.stale` khi `stale`.
- `snapshot` bị `fit.rs` cắt chuỗi > 1000 ký tự chỉ ở bước cuối; `limits` không có chuỗi dài nên an toàn.

## Test macOS / iPhone
- iPhone: máy có Claude Code với status line bridge bật, mở web app: số 5h / tuần phải bằng footer desktop (cùng % và giờ reset). Tắt bridge, không có số: không hiện hàng hạn mức, không hiện 0%.
- Máy chỉ có Codex: chỉ hiện Codex. Để > 30 phút không báo cáo mới: đánh dấu dữ liệu cũ.
