# HANDOFF 2026-11-11 10:17 — Orrery 4.0.x: phần chưa port sang Tauri (máy Windows làm tiếp)

Đọc file này là làm tiếp được. Trả lời tiếng Việt, gọi "chủ nhân", xưng "con". File này thay bản 08:57 (4.0.1 "Mất kết nối", bản cũ còn trong git history `faa36e3b`).

**Chủ nhân không test tay.** Session trên máy Windows tự build, tự chạy E2E, tự sửa lỗi, rồi báo kết quả.

## 1. Đang ở đâu
- Lỗi gốc: bản Tauri 4.0.x Explorer, Git Log (và Search, Changes, mở file…) trống. `core-desk` gọi ngược host `preview.*`; `apps/shell/src/hosts.rs` `call` không trả lời → `Err("no host for …")`. Quét thêm thì thấy khoảng 60 host call và 16 kênh trang (`not ported`) chưa có trên Tauri.
- Commit `ce7ff9b7` (đã push `master`) port hết. Bảng WP + chi tiết: `.planning/PROGRESS-2026-11-11-tauri-gaps.md`. Module mới trong `apps/shell/src/`:
  - Cổng thư mục: `preview_host.rs` (18 host `preview.*`), `preview_ipc.rs` / `preview_access.rs` / `preview_grants.rs` / `preview_files.rs` / `preview_watch.rs` (kênh `grant-folder`, `request-access`, `forget-grants`, `full-access`, `list-files`, `watch`, `unwatch`, `changes-watch`, `changes-unwatch`, `project-root`), `preview_text.rs`.
  - `preview_serve.rs` (server preview 127.0.0.1, token / root) + `files_ipc.rs` (`preview-url`, `open-path`, `reveal`, `pick-file(s)`, `logs`).
  - `host_run.rs` (`run.*`, `layout.urlFor` / `confirm`, `gate.dir` / `gate.file`), `host_devices.rs` (`devices.*`, `devfs.*`, `apk.pick`, `release.pick`, `capture.*`), `host_extra.rs` + `host_telegram.rs` (`telegram.*`, các confirm, `restore.*`, `phone.*`, `disk.open`).
  - `episodes.rs` (telegram / phone episode từ `core_serve::run_with`, `desk:tg-tick` 60 s), `lifecycle.rs` (`desk:run-boot` / `run-clean` / `tg-stop`, `bridge-migrate`), `desk.rs` `call_inside_allowing` (Telegram start run cho project chưa mở pane), `window.rs` `release_owners`.
  - `apps/core/crates/core-serve/src/lib.rs` thêm `run_with(cfg, emit)`; `apps/shell/Cargo.toml` thêm `notify` 8.2.
- Full access mặc định **bật** khi chưa có `preview-full-access.json` (chủ nhân chốt), file `{"full":false}` thì giữ tắt. E2E `cdp.cjs` ghi sẵn `{"full":false}` nên E2E vẫn chạy ở chế độ hỏi quyền như cũ.
- Web: 8 nút thiếu class gốc `ccm-btn` (nút "Cài…" XML Preview…) đã sửa.
- Test trên mac (đã chạy): `cargo test` shell 189 pass, core-desk 376 pass, core-serve pass; web 1914/1915 (`usePaneAttachments.test.ts` chập chờn khi chạy cả bộ, chạy riêng pass); `vue-tsc` / `tsc` sạch; `gen-desk-shim --check` khớp; `cargo build` debug OK.
- **Chưa chạy được E2E Tauri trên mac**: `cdp.cjs --shell=tauri` cần CDP của WebView2 (`--remote-debugging-port`), WKWebView không có → `no app page`. Phần chạy thật chưa ai kiểm.
- Nhánh `#[cfg(windows)]` chưa biên dịch lần nào: `phone.idle` (`GetLastInputInfo` qua user32), `filesystem_roots` (ổ `A:`–`Z:`), `log.rs`, path `\\?\` / so path không phân biệt hoa thường, `notify` (`ReadDirectoryChangesW`).
- Chưa commit (không đụng): `.planning/RELEASE-4.0.0.md` chủ nhân đang sửa dở.

## 2. Việc tiếp (theo thứ tự, trên máy Windows)
1. `git pull`, `npm ci`.
2. Test tự động: `cd apps/shell && cargo test -j 4`; `cd apps/core && cargo test -j 4 -p core-desk -p core-serve`; `npm run typecheck`; `npm test`. Lỗi biên dịch / test chỉ có trên Windows → sửa (đa số sẽ nằm trong các nhánh `cfg(windows)` ở trên).
3. Build cho E2E: `npm run build -w @ccm/web`, `node scripts/gen-desk-shim.mjs`, `cargo build -j 4 --manifest-path apps/shell/Cargo.toml` (ra `apps/shell/target/debug/orrery-shell.exe`).
4. E2E Tauri, mỗi file `node apps/desktop/e2e/<file>.cjs --shell=tauri` (instance cô lập port 4399, data trong temp). Ưu tiên theo phần vừa port:
   - `explorer`, `explorer-menu`, `log`, `diff`, `split-diff`, `file-editor`, `tabs-cwd`, `browser` (cổng thư mục, preview server, watch).
   - `run-pane`, `run`, `drawable-preview`, `devices`, `extensions`, `github-accounts`, `push`, `v70-tools`, `v72`–`v76`.
   - `apps/shell/e2e/t1.cjs` (child webview, capture).
   Ghi kết quả từng file (pass / fail) vào `.planning/PROGRESS-2026-11-11-tauri-gaps.md` mục QA. Fail do phần port → sửa trong module tương ứng ở trên, chạy lại. Fail do script chỉ viết cho Electron (selector / phím / `--user-data-dir`) → ghi lại, không sửa app cho khớp script.
5. Soi log khi chạy E2E: `<CCM_E2E_OUT>/electron.err` (stderr shell) không còn dòng `no host for` / `not ported`. Còn dòng nào → host / kênh đó vẫn thiếu, port tiếp theo cùng cách (host trong `host_*.rs`, kênh trang trong `ccm.rs` / `files_ipc.rs`).
6. Xong: cập nhật `.code_index/shell.md` nếu có sửa, báo chủ nhân qua Telegram (`telegram_send`, nếu project có topic) hoặc trả lời trong pane: bao nhiêu E2E pass, sửa gì, còn gì.
7. Release (chỉ khi chủ nhân bảo): tag `v4.0.1` đang trỏ `67e87fb2` (trước bản sửa này) → đề xuất lên `4.0.2` (bump `apps/desktop/package.json`, `package-lock.json`, `apps/web/src/lib/changelog.ts`, viết `.planning/RELEASE-4.0.2.md`), hoặc đặt lại tag `v4.0.1` nếu chủ nhân cho.

## 3. Quyết định và ràng buộc
- Commit / push / tag / release: chỉ khi chủ nhân nói rõ, mỗi lần tính riêng. Không thêm trailer attribution (`Co-Authored-By`, "Generated with Claude").
- Chủ nhân cho phép build + chạy E2E để test trong đợt này (không cần hỏi lại cho bước 2–5). Build release (`npm run dist:win`) chỉ khi cần cài thử hoặc chủ nhân bảo.
- Không viết comment trong code. "Tại sao" / bất biến → `.code_index/` hoặc `.planning/`.
- Full access mặc định bật; vẫn giữ lựa chọn đã lưu `{"full":false}`.
- Không đụng `.planning/RELEASE-4.0.0.md`.
- Ở bản release, mọi request trừ `/health` cần header `x-ccm-app` hoặc cookie `ccm_app` (+ fetch metadata, hoặc Origin cho `/ws`, xem `67e87fb2`).
- Quirk: rtk rewrite `cargo` → cần `$env:PATH` có `~/.cargo/bin` trước `cargo test`.

## 4. Còn treo (biết trước, không chặn)
- `ccm:open-external` chưa kiểm frame / URL không phải web.
- Toast của core-serve vẫn tắt trong shell (`toast_supported` cần host RPC).
- `device.used` không kèm tên AVD; `ccm:devfs-progress` gửi mọi cửa sổ; confirm luôn kiểu Warning, không đặt được nút mặc định.
- `ccm:logs` chỉ gồm `shell.log` + `pty.log`.
- Từ 4.0.0: OSC 633 trên Tauri; bash shim `--init-file`; Office khoá trên Tauri; W4 nút Office (chủ nhân chưa trả lời).

## 5. File cần đọc lại
`.planning/PROGRESS-2026-11-11-tauri-gaps.md`, `.code_index/shell.md` (mục Cổng preview, Host Devices, Host Telegram, Episode Telegram / phone + vòng đời, preview server), `apps/desktop/e2e/cdp.cjs` (`launch`, `SHELL`), `apps/shell/src/hosts.rs` `call`, `apps/shell/src/ccm.rs` `dispatch`.
