# Spike WP-50a — `core-remote-proto` → wasm, bắt tay qua relay local từ WebKit

Kết quả + số đo: `.planning/wp/v98/WP-50a-report.md`, `docs/qa/v98/WP-50a/{results,sizes}.json`.

## Cài một lần

- `rustup target add wasm32-unknown-unknown`
- `wasm-bindgen` CLI đúng bản crate trong `apps/core/Cargo.lock` (hiện 0.2.130): tải binary release
  `wasm-bindgen-0.2.130-x86_64-pc-windows-msvc.tar.gz` từ GitHub `wasm-bindgen/wasm-bindgen`, đặt
  `wasm-bindgen.exe` vào `scripts/spike-v98/wasm/.bin/` (hoặc có trên `PATH`).
- Relay: `cd apps/relay && npm install --workspaces=false --include-workspace-root=false`
- Thư mục này: `npm install` rồi `npx playwright install webkit`. Chromium dùng Chrome đã cài
  (`--channel=chrome`, mặc định); không có thì rơi về Chromium của Playwright (`npx playwright install chromium`).

## Lệnh

| Lệnh | Việc |
|---|---|
| `node build.mjs` | wasm release → `pkg/`, wasm debug + feature `test-vectors` → `pkg-vectors/`, peer native → `apps/core/target/debug/spike-peer.exe`; ghi `docs/qa/v98/WP-50a/sizes.json` |
| `node build.mjs --debug` | như trên nhưng `pkg/` cũng debug |
| `node vectors-node.mjs` | vector chéo wasm (Node) với `core-remote-proto/tests/vectors.json`, exit ≠ 0 nếu lệch |
| `node run.mjs` | cả chuỗi: build nếu thiếu, vector Node, `wrangler dev` (127.0.0.1:8787, state tạm), peer, trang tĩnh, Playwright WebKit rồi Chromium; in tóm tắt, ghi `results.json`, tắt hết tiến trình |

Tham số `run.mjs`: `--browsers=webkit,chromium` · `--iterations=20` (số lần 64 KiB) · `--port=8787` · `--channel=msedge` · `--build` (build lại).

## Thành phần

- `apps/core/crates/core-remote-wasm` — binding wasm-bindgen (dùng lại ở WP-54).
- `peer/` — desktop giả native (crate riêng, `[workspace]` rỗng): register Ed25519, WS `/m/<id>`, responder XXpsk3 + IK, `pair.ok` kèm `relayToken`, echo json / pty 64 KiB / ping, đóng `4401` khi key lạ.
- `web/` — trang tĩnh: nạp wasm, ghép qua relay bằng token mồi, nối lại bằng `relayToken`, đo thời gian.
- Token mồi `pair-bootstrap` do peer in ra thay cho luồng ghép qua relay của WP-52 (relay hiện bắt token cho mọi `/p/`).
