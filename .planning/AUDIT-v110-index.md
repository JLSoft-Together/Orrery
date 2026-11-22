# AUDIT v110 P0: cây index mới + CLAUDE.md gọn

Gói P0 của `.planning/PLAN-v110-modules.md`, 2026-11-22, chỉ đọc. Mục tiêu: 1 task đọc `lookup.md` + 1-2 file module (≤ 25 KB) thay vì 60-130 KB như hiện tại; `CLAUDE.md` nạp mỗi phiên còn khoảng 6 KB.

## 1. Đo hiện tại

`.code_index/` 804 KB (696 KB đang dùng + 107 KB `archive/`), 19 file, file nào cũng vượt 8 KB:

| File | KB | Nội dung | Đi đâu |
|---|---|---|---|
| `core.md` | 86.5 | Rust core theo WP (lịch sử v87) | tách 6 file module, phần WP sang `archive/core-v87.md` |
| `shell.md` | 79.1 | Tauri vỏ, remote, push, disk host | tách 4 file |
| `web.md` | 46.3 | web gốc + Office + map | tách theo feature |
| `_meta.md` | 43.7 | nhật ký thay đổi | giữ 30 dòng mới nhất (≤ 4 KB), phần cũ `archive/_meta-v90-v110.md` |
| `collector.md` | 39.7 | collector Node | `archive/` (P1 xoá collector) |
| `desktop.md` | 39.4 | Electron main | `archive/` |
| `features.md` | 38.8 | catalog tính năng | gọn còn ≤ 8 KB (1 dòng / tính năng + link plan), bản dài `archive/features-full.md` |
| `web-v71-v77.md` | 37.0 | web theo version | chưng cất vào file feature, rồi `archive/` |
| `desktop-desk-disk.md` | 36.6 | Electron disk + desk | `archive/` |
| `desktop-v75-core.md` | 31.9 | Electron v75-v87 | `archive/` |
| `desktop-v67-v74.md` | 31.0 | Electron v67-v74 | `archive/` |
| `web-v66-v70.md` | 30.3 | web theo version | chưng cất, `archive/` |
| `web-v79-v98.md` | 29.4 | web theo version | chưng cất, `archive/` |
| `protocol.md` | 28.9 | `packages/shared` | gọn ≤ 8 KB (bảng event + type chính), chi tiết sang `docs/protocol.md` |
| `web-pwa.md` | 26.2 | `apps/pwa` | `pwa.md` ≤ 8 KB |
| `web-workspace.md` | 25.3 | workspace, extensions, tests | chưng cất vào `web-workspace.md` mới |
| `web-v101-v102.md` | 23.3 | disk view, UI next gen | chưng cất vào `web-disk.md`, `web-next.md` |
| `core-disk.md` | 23.2 | crate core-disk | `core-disk.md` ≤ 8 KB |
| `archive/_meta-pre-v90.md` | 107.3 | | giữ |

Chưa có index: `apps/mobile` (Flutter + Rust, 35 000 dòng), `apps/relay`, `apps/layout-daemon` (chỉ README), `packages/mist-tokens`.

## 2. Cây mới (mỗi file ≤ 8 KB)

```
.code_index/
  README.md            cách dùng: đọc lookup.md → file module; quy tắc cập nhật (≤ 1 KB)
  lookup.md            việc → path::Symbol (≤ 8 KB, 60-80 dòng)
  _meta.md             30 thay đổi mới nhất
  features.md          catalog 1 dòng / tính năng
  protocol.md          packages/shared: event WS, ClientMessage, type chính
  rust-core.md         workspace apps/core: crate nào làm gì, bin `orrery-core` subcommand, build / test
  core-serve.md        core-serve + core-http + core-store (serve nhúng, cookie, persist)
  core-monitor.md      core-monitor + core-transcript + core-sources + core-procs (quan sát Claude / Codex)
  core-pty.md          core-pty (hub, viewers, jobs)
  core-desk.md         core-desk tổng quan + bảng module → file (kênh `ccm:*` đi đâu)
  core-desk-preview.md preview/* (files, git, explorer, gate)
  core-desk-run.md     run/*, layout/*, flutter_preview/*
  core-desk-devices.md devices/*, apk, capture, phone
  core-desk-telegram.md telegram/* (+ mt/)
  core-desk-mcp.md     mcp/*, mcp_desk/*, tools.json
  core-disk.md         crate core-disk
  core-toast-bridge.md core-toast, core-bridge (orrery-bridge)
  shell.md             Tauri vỏ: boot, cửa sổ, shim, `ccm` command, deep link, phát hành
  shell-hosts.md       host_*.rs, preview_*.rs, files_ipc, episodes, lifecycle, toast, power, badge
  shell-disk.md        disk_*.rs
  shell-remote.md      remote/* (gateway, relay, push, guard, chat)
  web-core.md          src/core (ws, desktop bridge, theme, settings, i18n, ui kit), tầng app
  web-workspace.md     features/workspace
  web-rail.md          features/rail
  web-next.md          features/next
  web-files.md         features/files
  web-git.md           features/git
  web-run.md           features/run
  web-agent-tools.md   features/agent-tools
  web-monitor.md       features/monitor + features/map
  web-disk.md          features/disk
  web-devices.md       features/devices
  web-remote.md        features/remote + features/telegram
  web-settings.md      features/settings + features/update + features/extensions
  web-small.md         click-menu, palette, usage, history, resources, deepseek
  office.md            features/office + packages/habblaud (chỉ đọc khi làm Office)
  pwa.md               apps/pwa
  mobile.md            apps/mobile (mới)
  relay.md             apps/relay + apps/layout-daemon (mới)
  archive/             lịch sử, không đọc khi làm task
    _meta-pre-v90.md  _meta-v90-v110.md  core-v87.md  shell-history.md  web-history.md
    collector.md  desktop.md  desktop-v67-v74.md  desktop-v75-core.md  desktop-desk-disk.md  features-full.md
```

40 file đang dùng, trung bình khoảng 5 KB → **khoảng 200 KB** (từ 696 KB). Lượng đọc mỗi task: `lookup.md` 8 KB + 1-2 module ≤ 16 KB.

## 3. Mẫu 1 file module (ví dụ `web-git.md`)

```markdown
# web-git: Diff, Git Log, Changes, commit, worktree

Đường vào: `apps/web/src/features/git/index.ts` (`ChangesPane`, `LogPane`, `useGitLog`, `openDiff`).
Phía desktop: kênh `ccm:git-*`, `ccm:changes-*` → `apps/core/crates/core-desk/src/preview/{changes,commit,log,branches}.rs` (xem `core-desk-preview.md`).

## File chính
| File | Vai trò |
|---|---|
| `components/ChangesPane.vue` | pane Changes, stage / discard / commit (1 170 dòng, nên tách) |
| `components/log/LogPane.vue` | Git Log: bảng commit, refs, filter |
| `lib/diffBlocks.ts::changeBlocks` | tách hunk cho DiffView / SplitDiff |
| `composables/useGitLog.ts::useGitLog` | nạp log theo trang, cache theo root |

## Bất biến
- Xoá branch chỉ sau confirm native (docs §11s).
- Push dùng tài khoản GitHub của repo (v58), token không bao giờ về trang.

## Liên kết
- Plan: `.planning/PLAN-v30-git-log.md`, `PLAN-v36-log-branches-toolbar.md`. E2E: `log`, `split-diff`, `v103-log-diff`.
- Phụ thuộc: `features/files` (`codeView`), `features/agent-tools` (turn changes).
```

Quy tắc viết: không kể lịch sử v-số (chỉ link plan), không lặp nội dung code, mỗi dòng có `path` hoặc `path::Symbol`, mục "Bất biến" chỉ chứa điều không được phá.

## 4. `lookup.md` (dòng mẫu thật)

```markdown
| Việc | Bắt đầu ở |
|---|---|
| Thêm kênh `ccm:*` từ trang | `apps/shell/src/ccm.rs::ccm` → `ccm.rs::dispatch` → `apps/shell/src/desk.rs::call_inside` → `apps/core/crates/core-desk/src/lib.rs::Desk::dispatch`; shim `scripts/gen-desk-shim.mjs` |
| Thêm message WS client → server | `packages/shared/src/types.ts::ClientMessage` + `apps/core/crates/core-http/src/client/mod.rs::ClientMessage` + `apps/core/crates/core-serve/src/dispatch.rs::client` |
| Thêm tool MCP `orrery` | `apps/core/crates/core-desk/src/mcp/tools.json` (đọc ở `mcp/defs.rs`), handler `mcp/t_*.rs`, vỏ `apps/shell/src/mcp.rs::start` |
| Deep link `agent-monitor://` | `apps/shell/src/links.rs::handle` |
| Cửa sổ chính / init script | `apps/shell/src/window.rs::create_main`, `window.rs::init_script` |
| Serve nhúng khởi động | `apps/core/crates/core-serve/src/lib.rs::run` (gọi từ `apps/shell/src/serve.rs`) |
| State pane / tab / split | `apps/web/src/stores/workspace.ts::useWorkspaceStore` |
| Chọn theme lúc mở app | `apps/web/src/lib/themes.ts::startTheme` (khớp `apps/web/index.html` `pick`) |
| Đọc menu lựa chọn Claude trên màn terminal | `apps/web/src/lib/termMenu.ts::parseTermMenu` |
| Kết nối WS, nhận snapshot / event | `apps/web/src/stores/connection.ts::useConnectionStore` |
| Settings lưu `ccm.*` | `apps/web/src/stores/settings.ts::useSettingsStore` |
```

Sau P3 các đường `apps/web/src/...` đổi sang `features/<feature>/...`; `lookup.md` viết lại ở P4, không viết trước.

## 5. `CLAUDE.md` gọn khoảng 6 KB

Đo hiện tại 26.1 KB: preamble 0.1, Project status 0.3, Commands 5.4 (dòng env dài 2.7), What this is 10.1, Architecture 2.3, Hard rules 6.5, Code index 1.1, MVP phase order 0.3.

| Giữ (đích KB) | Nội dung |
|---|---|
| Đầu file (0.6) | Orrery là gì trong 3 dòng (app desktop Tauri + Rust quan sát Claude Code / Codex, local only); target version Claude Code / Codex; link `docs/features-history.md` |
| Commands (1.4) | `npm install`, `npm run qa` (+ cờ), `npm test`, `npm run typecheck`, `npm run shell:dev`, `npm run dist`, E2E `node apps/shell/e2e/run.cjs`, `cargo test` cho `apps/core` / `apps/shell`; link `docs/env.md` |
| Kiến trúc (1.0) | cây `apps/shell`, `apps/core/crates`, `apps/web/src/{core,features}`, `packages/shared`, `apps/pwa`, `apps/mobile`, `apps/relay`, `apps/layout-daemon`, `packages/habblaud` (Office, lazy) |
| Nguyên tắc cốt lõi (1.2) | Observe → Normalize → Broadcast → Render, không tốn token; 6 ngoại lệ chủ nhân duyệt, mỗi cái 1 dòng + link docs § (§11x, §11am, §11an, §11w, §11as) |
| Luật cứng (1.4) | không bịa token; sanitize (ngoại lệ §11ar); không cuộn ngang; mọi chuỗi qua vue-i18n; màu qua token theo `.planning/COLOR-SYSTEM.md`; không blur ngoài `data-ui='next'`; không tải font từ mạng; chi tiết theme / sao / shape token → `docs/visual-style.md` |
| Index (0.4) | đọc `.code_index/lookup.md` trước task, cập nhật module + `_meta.md` sau task; "tại sao" ghi index / plan, không vào source |

| Chuyển đi | Đích |
|---|---|
| "What this is" dài (v28 → v102, 10 KB) | `docs/features-history.md` (mỗi version 1 đoạn, giữ link plan) |
| Danh sách env `CCM_*` (2.7 KB) | `docs/env.md`; sau P1 bỏ `CCM_CORE*=node`, `CCM_TOAST_BACKEND` |
| Theme / Galaxy / sao / meteor / shape token / Apple design system (khoảng 4.5 KB trong Hard rules) | `docs/visual-style.md` |
| Lịch sử đổi tên (Claude Code Monitor → Agent Monitor → GolAIm → Orrery) + prefix cũ giữ nguyên | `docs/features-history.md` đầu file (CLAUDE.md giữ 1 dòng: "`@ccm/*`, `CCM_*`, `ccm.*`, appId `dev.ccm.monitor`, data dir `ccm` giữ prefix cũ có chủ ý") |
| MVP phase order, Project status | bỏ (đã xong) |
| Câu "Before frontend work, load the skills …" | giữ 1 dòng |

Kết quả: khoảng 6 KB (giảm 77 %); tiết kiệm khoảng 20 KB mỗi phiên.

## 6. Thứ tự làm ở P4

1. Tạo `archive/`, dời `collector.md`, `desktop*.md` (sau P1).
2. Viết `rust-core.md`, `core-*.md`, `shell*.md` từ `core.md` + `shell.md` (Rust không đổi đường dẫn ở P1-P3, làm được sớm).
3. Viết `web-*.md` theo feature sau mỗi đợt P3 (đợt nào xong thì file đó viết ngay).
4. `lookup.md`, `README.md`, `features.md` gọn, `_meta.md` cắt.
5. `CLAUDE.md` + `docs/{env,visual-style,features-history}.md`.
6. Cập nhật skill indexer / hook nếu nó đọc tên file cũ (`web.md`, `desktop.md`, `collector.md`).
7. Đo lại (P5): KB index, KB CLAUDE.md, token đọc 1 task mẫu.
