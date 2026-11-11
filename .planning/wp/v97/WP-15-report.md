## WP-15 — PARTIAL (10/11 mục ĐẠT, mục 7 LỖI do F1)

### Đã đổi
- Mới `apps/mobile/rust/examples/phone_cli.rs` (325 dòng): "điện thoại" Rust dùng chính `rust_lib_orrery_remote::api::remote` (FRB API / crate proto). `pair <qr> --state <dir>` (ghép, lưu key + relayToken vào `phone.json`, không keychain), `repl` / `run <script.json|->` (phiên IK, JSON line vào / ra: `send`, `pty_in` có `claim`, `ping`, `quit`; ra `ready`, `json`, `pty`, `closed {code}`). Không sửa `Cargo.toml` (ws thuần; wss qua relay đi bằng tunnel Node).
- Mới `apps/shell/e2e/v97-remote.cjs` (~390 dòng): launch dev Tauri qua `cdp.cjs` (port 4419, CDP 9343, data dir tạm, `CCM_REMOTE_PORT=4330`, `CCM_E2E_CONFIRM=ok`, `CCM_CORE_BIN` = orrery-core debug), chạy 11 mục, ghi `docs/qa/v97/WP-15/RESULT.md`, chụp Settings.
- `scripts/qa-fast.mjs`: map `apps/shell/src/remote/` → `v97-remote`; tìm file E2E ở cả `apps/desktop/e2e` và `apps/shell/e2e`.
- `docs/qa/v97/WP-15/RESULT.md`, `settings-remote.png` (ảnh đầu tiên của trang Điều khiển từ xa, đã xem).
- Đã build lại: web dist (cũ 10/11), `orrery-shell` debug, `phone_cli`. Không đụng code sản phẩm.

### Bảng 11 mục (lần chạy cuối, 30 s, `node apps/shell/e2e/v97-remote.cjs`)
| # | Mục | Kết quả |
|---|---|---|
| 1 | Bật remote, IP LAN private, không bind 0.0.0.0 (netstat: chỉ `192.168.3.27:4330`) | ĐẠT 5/5 |
| 2 | Ghép QR → pair.ok → thiết bị trong `devices` (confirm bằng `CCM_E2E_CONFIRM=ok`) | ĐẠT 3/3 |
| 3 | IK + hello → welcome + snapshot + ws.snapshot khớp Pinia | ĐẠT 5/5 |
| 4 | ws.open shell: pane mới, `ui.view` và pane active không đổi | ĐẠT 6/6 |
| 5 | pty.attach, ready, `echo orrery-e2e` + claim → output, pane sống | ĐẠT 5/5 |
| 6 | Desktop ở Monitor: ws.open vẫn mở + pane `running` (H5) | ĐẠT 3/3 |
| 7 | ws.close pane bận → confirm.ask tới điện thoại, không `[role=alertdialog]`, answer ok → đóng | LỖI 7/8 (F1) |
| 8 | fs.list / fs.read trong cwd; ngoài root `denied`; UNC `invalid`; `..` `denied` | ĐẠT 5/5 |
| 9 | remoteRevoke → `device.revoked` + 4401; nối lại 4401 | ĐẠT 5/5 |
| 10 | Lặp 3–5 qua relay thật + thu hồi qua relay ra 4401 | ĐẠT 9/9 |
| 11 | Tắt remote → listener đóng, relay off (power request OS không kiểm được) | ĐẠT 3/3 |

### Lỗi tìm được
- F1 (trung bình, mục 7): lệnh dài gõ từ điện thoại không đặt `pane.ranCommand` nên `isBusy` = false, `ws.close` từ điện thoại đóng pane đang chạy `ping -n 60` mà không hỏi. Gõ trên desktop thì confirm đúng. Nghi: `apps/web/src/components/workspace/TermPane.vue:1036`, `apps/web/src/composables/usePaneInfo.ts:176`, `useRemotePhone.ts` (close).
- F2 (thấp): Settings hiện key thô `remoteV97.devices.platform.` khi platform ngoài android / ios. Nghi: `RemoteDeviceRow.vue:33`, `i18n/remote-v97.ts`.
- F3 (quan sát): sau ws.open, summary mới (pane có sessionId) tới sau ~3 s (một lần chạy > 8 s); `ws.patch` chưa kiểm. Nghi: `useRemoteSummary.ts`.

### Ngoài phạm vi / lưu ý
- Key Noise còn trong Credential Manager service `Orrery E2E v97`; máy E2E đã đăng ký trên relay thật.
- Dọn xong: không còn `orrery-shell` / `orrery-core` / webview2 mồ côi (baseline 13 webview2, `Orrery` 18828 và `orrery-core` 25276 của chủ nhân còn nguyên), port 4330 / 4419 / 9343 đóng, thư mục tạm xoá.
- Không chạy cargo test / clippy toàn bộ (ngoài phạm vi).

### Commit message đề xuất
test(remote): end-to-end Orrery Remote QA with a Rust phone client (pair, IK session, ws.open, pty attach, viewer fs, revoke, relay) against the dev Tauri app
