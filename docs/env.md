# Biến môi trường

Nguồn: `apps/core/crates/core-serve/src/config.rs::ServeConfig` (serve), `apps/shell/src/config.rs::ShellConfig` (vỏ Tauri), từng crate core-desk. Prefix `CCM_*` giữ từ tên cũ có chủ ý. Từ v110 không còn `CCM_CORE*`, `CCM_PTY_*`, `CCM_TOAST_BACKEND=electron`: serve và terminal luôn chạy bằng Rust.

## Serve (`orrery-core serve`, chạy nhúng trong vỏ)

| Biến | Ý nghĩa |
|---|---|
| `CCM_APP_KEY` | khoá bắt buộc, header `x-ccm-app` hoặc cookie `ccm_app` cho mọi request trừ `/health`; thiếu khoá và không có `--dev` thì serve thoát |
| `CCM_PORT` | cổng WebSocket / HTTP (mặc định `4317`; bản debug của vỏ `4419`; `0` = cổng trống) |
| `CCM_DEV_ORIGIN_PORT` | cổng Vite được phép làm Origin khi `--dev` |
| `CCM_DATA_DIR` | `%LOCALAPPDATA%/ccm`, macOS `~/Library/Application Support/ccm`: cache usage, aliases, `timeline/`, `disk/`, `capture.json`... |
| `CLAUDE_CONFIG_DIR` | gốc Claude Code (`~/.claude`) |
| `CODEX_HOME` | gốc Codex (`~/.codex`); `CCM_CODEX=0` tắt Codex Monitor |
| `CODEWHALE_HOME` | gốc DeepSeek CLI (`~/.codewhale`); `CCM_DEEPSEEK=0` tắt DeepSeek Monitor |
| `CCM_DEEPSEEK_FEATURE=1` | bật lại tính năng DeepSeek đang tạm dừng (pane, Settings, `open_session` agent `deepseek`, `hand_back`); mặc định tắt từ 2026-11-03 |
| `CCM_STALE_TTL_MIN` | 120: session cũ hơn bấy nhiêu phút bị dọn |
| `CCM_PEER_TTL_MIN` | 60: link session với session hết hạn sau bấy nhiêu phút yên lặng |
| `CCM_VERIFY_PROCESSES=0` | không kiểm process sống (dùng với `scripts/demo-fixture.mjs`) |
| `CCM_WEB_DIST` | thư mục build web (vỏ tự chọn từ resources) |
| `CCM_TOAST=0` | tắt toast / alert; `CCM_TOAST_LINK` = prefix deep link (`agent-monitor://`) |
| `CCM_STATUSLINE_BRIDGE`, `CCM_STATUSLINE_COMMAND`, `CCM_BRIDGE_BIN` | đường tới `orrery-bridge` (statusline / hook) |
| `CCM_HOST_RPC=1` | serve gọi ngược vỏ qua `host.req` / `host.res` thay vì `NativeDeps` |
| `CCM_SERVE_STATS=1` | in thời gian khoá store / process table ra stderr lúc thoát |
| `CCM_RES_FIXTURE` | fixture cho Resources (test) |

## Vỏ Tauri (`apps/shell`)

| Biến | Ý nghĩa |
|---|---|
| `CCM_SHELL_WEBVIEW_DIR` | thư mục dữ liệu WebView2 (E2E dùng thư mục tạm) |
| `CCM_SHELL_LOG=0\|1` | ép tắt / bật ghi stderr vào `<data>/shell.log` |
| `CCM_KEYRING_SERVICE` | tên mục keychain (`Orrery`, debug `Orrery Dev`, E2E `Orrery E2E`) |
| `CCM_CORE_BIN` | đường tới binary `orrery-core` (pty / procs); mặc định cạnh exe hoặc `resources/core/` |
| `CCM_UPDATE_FEED` | feed cập nhật (bản debug) |
| `CCM_REMOTE_PORT` | cổng LAN của Remote (4320) |
| `CCM_LAYOUT_JAR`, `CCM_LAYOUT_KIT`, `CCM_LAYOUT_LOOSEKIT`, `CCM_LAYOUT_OVERRIDE`, `CCM_LAYOUT_LOG`, `CCM_LAYOUT_INIT` | kit XML Layout Preview (`apps/layout-daemon/kit.json`) |
| `CCM_ELECTRON_USER_DATA`, `CCM_ELECTRON_KEYCHAIN_SERVICE` | chỉ để nhập dữ liệu từ bản Electron cũ (`apps/shell/src/legacy.rs`) |
| `ORRERY_SHIM_DIR` | thư mục shim `window.ccmDesktop` (test) |

## Trong pane terminal

`ORRERY_API`, `ORRERY_TOKEN`, `ORRERY_MCP_CONFIG` (MCP `orrery`, token riêng mỗi pane), `ORRERY_SI_NONCE` (shell integration OSC 633). Process `orrery-core` pty chỉ nhận env trong `CORE_ENV_KEYS` (`apps/shell/src/pty.rs`); env của từng pane do `apps/shell/src/presets.rs` dựng (bỏ `CLAUDE_CODE_*` / `CODEX_*` của phiên cha).

## Khác

- `CCM_DOCUMENTS_DIR`, `CCM_USER_DATA`, `CCM_APP_VERSION`, `CCM_LIBS_REGISTRY`, `CCM_OSV_URL`, `CCM_GH_API`, `CCM_TELEGRAM_URL_OVERRIDE`, `CCM_PHONE_URL_OVERRIDE`, `CCM_VIDEOS_DIR`, `CCM_AGENT_DATA_RECOVER`: `apps/core/crates/core-desk/src/lib.rs` và module tương ứng (chủ yếu cho test / E2E).
- E2E: `CCM_E2E_PORT` (4399), `CCM_E2E_DBG` (9333), `CCM_E2E_OUT`, `CCM_E2E_SHELL_EXE`, `CCM_E2E_CONFIRM=ok|cancel`: `apps/shell/e2e/cdp.cjs`.
- Dev: `npm run dev` chạy `orrery-core serve --dev` (cổng `127.0.0.1:4317`, không app key, không terminal). Đặt `CCM_DATA_DIR` riêng để timeline demo không lẫn vào dữ liệu thật.
