# WP-v87-2-B — báo cáo

**Trạng thái: PARTIAL** — mọi file xong, nhưng `cargo test` / build Rust **CHƯA CHẠY ĐƯỢC** trên máy này: không có MSVC Build Tools (không `link.exe`, không Windows SDK, không gcc). Code Rust chỉ được review bằng mắt, chưa biên dịch. Cần chủ nhân cài "Visual Studio Build Tools 2022 + Desktop development with C++" (con không tự cài: thay đổi hệ thống ngoài scope) rồi chạy `cargo test --locked -j 6` trong `apps/core` bằng PowerShell.

## Đã đổi

| File | Việc |
|---|---|
| `apps/core/Cargo.toml`, `rust-toolchain.toml` (stable, minimal), `Cargo.lock` (21 package, sinh bằng `cargo generate-lockfile`) | Workspace `core-pty` + `core-bin`, release: thin LTO, `panic=abort`, strip |
| `apps/core/crates/core-pty/src/control.rs` | NDJSON `MainToCore` (init / census / kill-all / shutdown / Unknown), `CoreToMain::Listening`, `parse_line`, `encode`; 4 unit test, 1 test đọc `apps/desktop/test/fixtures/pty-control.json` |
| `crates/core-pty/src/{lib,session,ring,flow,ws,owners}.rs`, `jobs/{mod,windows,unix}.rs` | Module khung (rỗng, cho WP-C/D) |
| `crates/core-bin/src/main.rs` | Stub `orrery-core`: đọc `init` → bind `127.0.0.1:0` → in `listening` → thoát 0 khi `shutdown` / stdin EOF; dòng đầu không phải `init` → exit 2; stdin đóng trước init → 0 |
| `crates/core-bin/tests/stub.rs` | 4 integration test (init→listening→connect TCP→EOF→exit 0; shutdown; init sai → 2; EOF trước init) |
| `apps/desktop/build.mjs` (import :4, khối `buildCore` trước `const zipOnly`, gọi `buildCore(dist)` trước `rmSync(STAGE)`) | `cargo build --release --locked -p core-bin -j min(JIN_MAX_THREADS,6,cpu)`; tìm cargo trên PATH rồi `~/.cargo/bin`; dev: chỉ build khi thiếu `target/release/orrery-core(.exe)`, thiếu cargo / build lỗi chỉ cảnh báo; `--dist` bắt buộc (exit 1 khi lỗi), macOS build 2 triple + `rustup target add` nếu thiếu, chép vào `apps/core/target/bundle/<arch>/`; `CCM_SKIP_CORE_BUILD=1` bỏ qua (dist: cần binary sẵn, thiếu thì exit 1). `--daemon-zip-only` thoát trước `buildCore` nên job `daemon` ubuntu không cần Rust |
| `apps/desktop/electron-builder.yml` (:59-61 mac, :71-72 win) | `extraResources` `../core/target/bundle/${arch}/orrery-core(.exe)` → `core/` |
| `.gitignore` | `apps/core/target/` |
| `.github/workflows/desktop.yml` | job build: `dtolnay/rust-toolchain@stable` (+ 2 target mac), `Swatinem/rust-cache@v2` (`apps/core`) |
| `.github/workflows/test.yml` | rust toolchain + cache + bước `cargo test --locked -j 4` (win + mac) |
| `.code_index/core.md` | Khung mới |

`scripts/dist.mjs`, `daemon-kit.mjs`, `package.json` root: không cần đổi.

## Tiêu chí B: 5/7 (2 chưa chứng minh được bằng chạy thật)

| Tiêu chí | Kết quả |
|---|---|
| Workspace + toolchain pin + Cargo.lock | ✓ |
| Stub init → listening → thoát khi stdin EOF / shutdown | viết xong, **chưa biên dịch** (không có linker) |
| `cargo test --locked -j 6` xanh | **CHƯA CHẠY** |
| `buildCore()` (dev warn / dist bắt buộc / skip) | ✓ nhánh warn + skip chạy thật |
| `electron-builder` extraResources | ✓ YAML parse ok; `npm run dist:*` không chạy (luật) |
| `.gitignore` + CI | ✓ YAML parse ok; workflow chưa chạy thật trên runner |
| `.code_index/core.md` | ✓ |

## Audit / Review / Test

- `cargo test -j 6` (PowerShell + Git Bash) → fail ở build script `serde`... : `linker link.exe not found` (PowerShell), Git Bash còn bị `link` của coreutils che. Kiểm: không có `link.exe` / Windows Kits / gcc. Target cài sẵn chỉ có android + msvc. `cargo generate-lockfile` → ok.
- `node --check apps/desktop/build.mjs` → ok.
- `CCM_SKIP_DAEMON_BUILD=1 node build.mjs` trên máy này (không linker) → in `build: WARNING apps/core build failed ...`, exit 0, esbuild vẫn chạy tiếp. Cùng lệnh thêm `CCM_SKIP_CORE_BUILD=1` → không cargo, exit 0. `target/` đã xoá sau test.
- `js-yaml` parse 3 file: `desktop.yml`, `test.yml`, `electron-builder.yml` → ok. Giữ CRLF của `test.yml` / `electron-builder.yml` / `.gitignore`.
- Review edge case: máy không cargo → warn (dev) / exit 1 (dist); binary thiếu lúc dev → build một lần rồi warn nếu lỗi; CI mac 2 arch: macos-latest (arm64) cross build `x86_64-apple-darwin` (target cài bằng action + `ensureRustTarget` dự phòng); dist trên Windows chỉ `x64`; chmod 755 khi stage.
- Rủi ro đã nhận: code Rust có thể có lỗi biên dịch nhỏ chưa thấy (đặc biệt `#[serde(other)]` trên enum tag nội bộ, `tokio::select!` với `lines.next_line()`); sửa khi có linker. `npm test -w @ccm/desktop` / typecheck không chạy lại: WP B không đụng `src/**`.

## Kết luận kỹ thuật

- **`${arch}` trong extraResources: hỗ trợ.** `app-builder-lib/out/fileMatcher.js:42-43` chạy `macroExpander` trên `from` và `to`; macro `arch` = `Arch[arch]` = `x64` / `arm64` (khớp tên thư mục `target/bundle/<arch>/`, nên không cần override trong `dist.mjs`). Triple Rust khác tên arch nên build.mjs stage về thư mục theo `<arch>`.
- **npm workspaces `apps/*`: không vỡ.** `apps/core` không có `package.json`; `npm ls --workspaces` liệt kê bình thường, không lỗi, không cần sửa root.
- Git Bash trên máy này: `link` của coreutils che `link.exe`; cargo chạy bằng PowerShell / VS dev prompt.

## Commit message đề xuất

🦀 Scaffold apps/core Rust workspace (core-pty skeleton + orrery-core control stub), buildCore in build.mjs, core/ extraResources per arch, rust steps in CI
