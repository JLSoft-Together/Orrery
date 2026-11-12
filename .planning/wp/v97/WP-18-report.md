## WP-18 — DONE

### Đã đổi
- F1: `apps/shell/src/remote/phone_pty.rs` (`typed()`: input pty có `\r`/`\n` → `Host::ask("remote.typed", {device, sessionId})`, gap 2 s), `pty_bridge.rs` (`Attach.session`, `typed_at`); `packages/shared/src/orreryApi.ts` (action `remote.typed`, source `phone`); `apps/web/src/composables/useRemotePhone.ts` (`typed()` đặt `ranCommand` cho pane shell có `sessionId` đó). Ngoài danh sách phạm vi nhưng bắt buộc: `packages/shared/src/orreryApi.ts` + type `RemoteDevice.platform` trong `remote.ts`.
- F2: `devices.rs` (`known_platform`, `set_platform` lưu `unknown`), `status.rs` (`device_json` chuẩn hoá), `RemoteDeviceRow.vue` + `i18n/remote-v97.ts` (`platform.unknown`: Unknown / Không rõ).
- Log: dòng log trước hiện device id (`e2e-30dd…`), nay hiện tên thiết bị (`useRemotePhone.ts` `log`). Log có ghi mở / đóng (`ws.open`, `ws.close`) — ảnh xác nhận.
- E2E `apps/shell/e2e/v97-remote.cjs`: bỏ polling `ws.get` + sleep 3 s, theo dõi `ws.snapshot` + `ws.patch` (áp JSON patch), thêm mục 12 (F3, log, F2, chụp Settings), kết quả ghi `docs/qa/v97/WP-18/`.
- Test mới: Rust `a_phone_views_and_types...` (assert `remote.typed`), `a_platform_outside_android_and_ios...`; Vitest `useRemotePhone.test.ts` (ranCommand).
- `.code_index/{_meta,shell,web,protocol}.md`.

### Nguyên nhân F3
Không phải lỗi sản phẩm: `paneOnPhone` của E2E gọi `ws.get` rồi `sleep(3000)` giữa các lần thử nên đo ra ~3 s (có lần > 8 s). Đường thật: renderer debounce 200 ms → ` remote-publish` → Board → `ws.patch`; đo mới: `ws.open` gửi → pane mới có trong summary điện thoại sau 324–327 ms (qua `ws.patch`), sessionId có ngay sau đó (pane đã `running`). Không sửa code sản phẩm cho F3.

### Tiêu chí 4/4
- E2E `node apps/shell/e2e/v97-remote.cjs`: 12/12 mục ĐẠT (mục 7 nay 8/8; mục 12: F3 ≤ 1 s qua ws.patch, log mở + đóng, Settings ≥ 2 dòng, F2 "Không rõ"). Chạy 2 lần, 2 lần đạt. `docs/qa/v97/WP-18/RESULT.md`.
- `cargo test --locked -j 3 remote`: 66/66 (gồm test hub thật). `npm run typecheck -w @ccm/web` và `-w @ccm/shared` sạch. Vitest (useRemotePhone, useRemoteSummary, remoteParse, settings): 39/39.
- Ảnh `docs/qa/v97/WP-18/settings-remote-log.png` (+ `settings-remote.png`), đã mở xem: log 3 dòng với tên thiết bị, hàng thiết bị "Không rõ", không key thô.
- Không đụng app thật (port 4330 / 4419 / 9343 tạm, data dir tạm, `orrery-core` 25276 của chủ nhân còn nguyên, không process mồ côi). Không comment. Không commit.

### Lưu ý
- `ws.send` log chưa chạy trong E2E (cần pane agent); logic cùng đường `log()`.
- E2E script 465 dòng (script có sẵn 405).

### Commit message đề xuất
fix(remote): mark a shell pane busy when a phone types a command, show unknown phone platforms and device names in the log, and measure ws.patch latency in the E2E
