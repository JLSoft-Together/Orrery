# HANDOFF 2026-11-10 23:30 — v87 bước 6 trên Mac

Thay `HANDOFF-2026-11-10-2100-v87-step6.md`. Nhật ký: `.planning/PROGRESS-2026-11-09-v87.md`.

## Trạng thái
- Mac bước 1–3 của handoff 21:00 xong, sửa 5 lỗi chỉ có trên mac (xem `.code_index/_meta.md` dòng "v87 mac").
- Gói mới (đều không build release, không E2E; cờ mặc định ghi bên cạnh):
  - D2 PARTIAL — `github`, `phone`, `capture`, `power`, `disk`, `agentStatus`, `resources` → `core-desk`, mặc định node (`DESK_NODE_DEFAULT`). `.planning/wp/WP-v87-6-D2-report.md`.
  - D3b DONE — 11 kênh previewTools (format, check-run/cancel, perm-*, sensitive-deny, ctx-sources, memory-doctor, http-send/cancel, replace-files qua `regress`), mặc định rust. `WP-v87-6-D3b-report.md`.
  - D4 PARTIAL — 23 kênh `ccm:run-*` + 5 `ccm:fwp-*`, mặc định node; chặn bật rust: layout model / telegram / runLogs / MCP run tools / settings đọc `runManager` TS → cần cầu nối từ `ccm:run-event`. `WP-v87-6-D4-report.md`.
- Cổng gộp (main kiểm 23:30): `cargo test` 694/0, clippy sạch, desktop 1617 pass / 29 skip, typecheck 0.
- Repo Orrery: **chưa commit** (auto mode chặn lệnh commit đầu phiên; chủ nhân chưa nói commit Orrery). jin-workflow đã commit + push (`cec3ea6`, `d0c7f76`).

## Việc tiếp
1. Chủ nhân nói "commit" → commit Orrery theo batch (mac fixes; D2; D3b; D4), không trailer.
2. Chủ nhân build: `cd apps/core && cargo build --release -j 4 -p core-bin -p core-bridge` + `node apps/desktop/build.mjs`.
3. Sau build, E2E (rust rồi node), lệnh trong từng báo cáo: D1 (`v74`, `extensions`, `tier2`, `run-pane`, `v77b`), D2 (`github-accounts`, `push`, `v76`, `v77a`, `agent-api`, port 4492), D3b (`v70-tools`, `v72`–`v76`, `file-editor`), D4 (port 4511, `rg -l "ccm:run-|ccm:fwp-" apps/desktop/e2e`). Qua → bỏ module khỏi `DESK_NODE_DEFAULT`.
4. D4b: cầu nối `ccm:run-event` → runManager TS consumers; rồi D5–D9, T2, T3 theo `PLAN-v87-step6-tauri.md`.
5. Chủ nhân test tay `npm run desktop` (checklist `.planning/CHECKLIST-v87-step6-mac.md`).

## Ghi nhớ
- `cargo` ở `~/.cargo/bin`; `.zshrc` đã source `~/.cargo/env` (session mới mới có). Session cũ: `export PATH="$HOME/.cargo/bin:$PATH"`.
- Hook resource-guard đã đọc RAM bằng `vm_stat` (trước luôn báo 91–96% trên mac).
- Vỏ Tauri phải: gọi `set_sink` của `Emitter` (event awake / disk / agent status), host `keychain.*`, `run.owner` / `run.confirm` / `run.adbUse`, `preview.confirm` kind `http`.
