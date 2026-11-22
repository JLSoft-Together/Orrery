# pwa: apps/pwa (@ccm/pwa), web app iPhone / iPad / trình duyệt điện thoại

Đường vào: `apps/pwa/src/main.ts`, `apps/pwa/src/App.vue`, `apps/pwa/src/router.ts` (router tự viết theo history). Build Vite base `/app/`, dist đưa vào relay `/app/*` (`scripts/relay-web.mjs`).

## File chính

| File | Vai trò |
|---|---|
| `apps/pwa/src/remote/noise.ts` | Noise qua wasm `core-remote-wasm` |
| `apps/pwa/src/remote/session.ts` | `RemoteSession`: request / event / keepalive |
| `apps/pwa/src/remote/connection.ts` | `MachineConnection`: backoff, nền / trước, `4401` -> thu hồi |
| `apps/pwa/src/remote/pairing.ts` | pair QR |
| `apps/pwa/src/remote/hub.ts` | `RemoteHub`: nhiều máy, nhiều kết nối |
| `apps/pwa/src/storage/keyStore.ts` | khoá AES-GCM (hook WebAuthn) |
| `apps/pwa/src/storage/repos.ts` | `MachineRepo`, cache snapshot IndexedDB |
| `apps/pwa/src/lib/monitorModel.ts` | parse snapshot, lọc, nhóm |
| `apps/pwa/src/lib/workspaceModel.ts`, `viewerModel.ts` | state Workspace / viewer pane |
| `apps/pwa/src/lib/chatModel.ts`, `chatFeed.ts` | chat view (Claude / Codex) |
| `apps/pwa/src/terminal/ptyStreams.ts` | luồng byte terminal, `fitLayout.ts` (claim kích thước) |
| `apps/pwa/src/push/webPush.ts` | Web Push (RFC 8291) |
| `apps/pwa/src/views/` | `HomeView`, `PairView`, `MonitorView`, `WorkspaceView`, `ChatView`, `TerminalView`, `SettingsView`... |
| `apps/pwa/vite/serviceWorker.ts` | sinh `sw.js` (precache theo hash, skip-waiting chỉ khi có message) |

## Chạy / test

- wasm: `npm run wasm -w @ccm/pwa` (-> `apps/pwa/src/gen/`, gitignored). E2E: `npm run e2e -w @ccm/pwa` (`apps/pwa/e2e/run.mjs`, peer giả bằng Rust). Gate điện thoại: `node scripts/qa-remote.mjs` (script `qa-remote` trong `scripts/jin-scripts.json`).

## Bất biến

- Khoá thiết bị niêm phong ở IndexedDB; quên máy thì xoá + tombstone chặn lưu muộn.
- Không tràn ngang ở 375 / 390 / 430 px; chat là mặc định; ưu tiên mượt, phản hồi nhanh.
- Mọi dữ liệu qua kênh Noise end-to-end; relay chỉ chuyển byte đã mã hoá.

## Liên kết

- Gateway: `shell-remote.md`; relay: `relay.md`; docs `docs/orrery-remote.md`, `docs/remote-integration.md`. Plan: `.planning/PLAN-v98-ios-pwa.md`. Bản chi tiết cũ: `.code_index/archive/web-pwa-full.md`.
