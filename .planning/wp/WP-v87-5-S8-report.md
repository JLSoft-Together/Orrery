# WP-v87-5-S8 — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S8.md`, dòng S8 plan `PLAN-v87-step5-core-store.md` (§3.6 mới). Không commit / push, không sửa `apps/collector/src`, `apps/web`, `packages/shared`, không đụng file của S5 / S7. Không chạm app Orrery đang chạy (không mở cổng 4317, không đọc `%APPDATA%\Agent Monitor` / `%LOCALAPPDATA%\ccm`). CPU lúc chạy ~26 %.

## Đã làm

1. **Protocol `host.*`** (ghi đầy đủ plan §3.6): NDJSON, Rust → main qua stdout `{"type":"host.req","id","method","params"}`, main → Rust qua stdin `{"type":"host.res","id","result"|"error"}`. Method: `focus {pid}`, `open {dir,app}`, `launch {dir,mode,app?}`, `pickFolder {}`.
2. **Rust** (`apps/core/crates/core-monitor/src/host/`):
   - `protocol.rs`: `valid_pid` (>= 5), `valid_dir` (không rỗng, <= 1024 UTF-16, không ký tự điều khiển, tuyệt đối), deadline chờ (focus 25 s, open 20 s, launch 70 s, pickFolder 10 phút 15 s), parse response.
   - `client.rs` `HostClient`: ghi stdout, chờ response theo id (`call` có deadline), `on_message` cho dòng stdin, `close` đánh thức mọi chỗ đang chờ.
   - `deps.rs` `HostDeps` = `SystemDeps` + 4 hook `launch|focus_window|open_folder|pick_folder` qua RPC. Pid / dir sai không gửi đi (focus `notFound`, open `notFound`, launch `failed`); timeout / lỗi remote -> `failed`; kênh đóng -> `None` (= `unsupported` như S4); dir từ `pickFolder` kiểm lại, sai -> `failed`; kết quả lạ -> `failed`.
   - `serve/mod.rs`: `ServeConfig.host_rpc` (`CCM_HOST_RPC=1`, mặc định tắt nên `monitor-serve` / harness S4 không đổi hành vi), bật thì dùng `HostDeps`, stdin `Message` -> `HostClient::on_message`, EOF đóng client. `lib.rs` thêm `pub mod host`. Không đụng `Cargo.toml` / `Cargo.lock`.
3. **Desktop main** `apps/desktop/src/host-rpc.ts` `HostRpc.handleLine / handle`: validate lại pid (5..2^32-1), dir (tuyệt đối, <= 1024, không ký tự điều khiển), app / mode / terminalApp theo danh sách, method lạ -> `error: bad request` và không handler nào chạy. Tái dùng nguyên `collector/src/{focus,opener,launcher,folder-pick}.ts` (`systemHandlers`, PowerShell / osascript giữ nguyên). `focus` chạy từng cái một, `pickFolder` đang mở dialog thì trả `busy` (như `Monitor` Node), dir `ok` phải hợp lệ, lỗi launch map về `notFound|denied|notInstalled|failed` đúng như `monitor.ts`.
4. Docs: plan §3.6 + dòng S8, `.code_index/{core,desktop,collector,_meta}.md`.

## File (khớp `git status`, phần của S8)

- Mới: `apps/core/crates/core-monitor/src/host/{mod,protocol,client,deps}.rs`, `apps/core/crates/core-monitor/tests/host_rpc.rs`, `apps/desktop/src/host-rpc.ts`, `apps/desktop/test/host-rpc.test.ts`, báo cáo này.
- Sửa: `apps/core/crates/core-monitor/src/lib.rs` (+1 dòng), `apps/core/crates/core-monitor/src/serve/mod.rs`, `.planning/PLAN-v87-step5-core-store.md`, `.code_index/{core,desktop,collector,_meta}.md`.
- Không phải của S8 (S5 / S7 / có sẵn): `agentprocs.rs`, `cliprocs.rs`, `codex/`, `fswatch.rs`, `core-toast/`, `.planning/PROGRESS-*`, `RELEASE-v2.3.0.md`, `wp/WP-v87-5-S5|S7*`, `docs/qa/*`. Test desktop làm đổi EOL file `apps/desktop/test/__snapshots__/fwp-host.test.ts.snap`, đã `git checkout` lại (trước đó sạch). Mọi file mới <= 145 dòng, không comment.

## Test (số thật)

- `cargo test --locked -j 4` (cả workspace lúc chạy, gồm crate S5 / S7 đang dựng dở nếu đã vào tree): tất cả pass, 0 failed, 6 ignored. Phần S8: unit `host::protocol` 2 + `host::client` 2 + `tests/host_rpc.rs` 6 = 10 test mới; `core-monitor` tổng 48 sau khi thêm.
- `cargo clippy --locked -j 4 --all-targets -- -D warnings`: sạch.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: **131 file passed, 1533 passed, 18 skipped** (gồm `host-rpc.test.ts` 10 test).
- `npm run typecheck -w @ccm/desktop`: exit 0.
- `npm test -w @ccm/collector -- --maxWorkers=4`: **49 file passed + 1 skipped, 526 passed, 3 skipped** (không đổi so với S4).

## Parity / khác biệt biên

- Cổng parity của dòng S8 là "kiểm thủ công" và brief bỏ kiểm thủ công: chỉ có test tự động (fake main trong Rust, fake handler trong TS). Chưa chạy thật Win + mac focus / open / launch / pick; test `focus-mac` hiện có giữ nguyên và pass.
- Hook `Deps` của S4 đồng bộ, nên thread monitor đứng chờ trong lúc dialog chọn folder đang mở (tối đa 10 phút 15 s); Node không đứng. Chấp nhận được cho giờ, S10 nên chuyển `pick_folder` sang thread riêng nếu thấy ảnh hưởng.
- Hàng đợi `focusing` nằm ở main (Rust gọi tuần tự sẵn); `busy` của pick nằm ở main, không ở Rust.
- `main.ts` chưa đụng: chưa có `CoreServeHost` để nối, nối ở S10.
- Trang web không có đường gọi `host.*`: chỉ `Monitor` gọi, pid lấy từ tracker, dir từ cwd tracker / favorites; main validate lại.

## Việc cho WP sau

- S10: `CoreServeHost` đọc từng dòng stdout, `host.req` -> `new HostRpc().handleLine(line)` -> ghi dòng trả về vào stdin của `serve`; đặt `CCM_HOST_RPC=1` trong `collectorEnv`; cân nhắc `pick_folder` thread riêng.
- S5: `focus_window` / `open_folder` của Codex / DeepSeek đi qua cùng `Deps` nên tự dùng RPC.
- S11: kiểm thủ công Win + mac (focus, mở folder, launch, chọn folder) trong E2E.

STATUS: DONE
