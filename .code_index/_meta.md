# _meta: thay đổi gần đây

Mỗi dòng 1-2 câu: gì, rồi file / symbol chính. Lịch sử trước v110: `archive/_meta-pre-v110.md` (đường dẫn trong đó có thể đã chết: collector, desktop, web components / lib / stores cũ).

- 2026-11-22 v110 rà index: sửa cột Vai trò `core-desk-mcp.md` (`t_files.rs`, `device_screenshot` ở `t_run.rs`), `core-disk.md`, `core-serve.md`, `core-desk-telegram.md`; path web cũ trong `docs/*.md` + `bug_pool/` đổi theo `git mv`.
- 2026-11-22 v110 P4: index mới (`.code_index/lookup.md` + 1 file mỗi module, tối đa 8 KB), `CLAUDE.md` gọn, tài liệu dài sang `docs/features-history.md`, `docs/core-principle.md`, `docs/env.md`, `docs/visual-style.md`; kiểm bằng `scripts/index-check.mjs`; log E2E `shell.log` trong `apps/shell/e2e/cdp.cjs`.
- 2026-11-22 v110 P3: `apps/web/src` chia `core/` + `app/` + `features/<feature>/` (alias `@core`, `@features`), luật `apps/web/src/importRules.test.ts`, codemod `scripts/web-move.mjs`; còn feature -> app 44 cạnh (chủ yếu `app/stores/connection.ts`).
- 2026-11-22 v110 P2: Office tách `apps/web/src/features/office/` lazy, event qua `apps/web/src/core/stores/connectionEvents.ts`, i18n Office nạp khi mở.
- 2026-11-22 v110 P1: xoá Electron (thư mục desktop cũ) + collector Node (thư mục collector cũ); E2E sang `apps/shell/e2e/`, version `apps/shell/package.json`, shim nguồn `apps/shell/shim/preload.ts`, `npm run dev` = `orrery-core serve --dev` + Vite.
- 2026-11-22 v110 P0.5: parity Rust cho mở / focus ngoài app (`apps/core/crates/core-monitor/src/native/mod.rs`), macOS PATH login shell + menu + Move to Applications (`apps/shell/src/mac_env.rs`, `mac_menu.rs`, `mac_move.rs`), Dock bounce + toast nút (`apps/shell/src/attention.rs`, `menu_toast.rs`); macOS chưa compile trên Mac.
- 2026-11-21 v109: legacy theme sau "View all" (`apps/web/src/core/lib/themes.ts`), secret guard (`apps/core/crates/core-desk/src/preview/secrets.rs`), shell marks OSC 633 (`apps/core/crates/core-pty/src/marks.rs`), chat view, share mode, term search, shortcuts, footer sys, diff comments.
- 2026-11-20 v108: bảng màu UI next (Ngẫu nhiên, Tự chọn, Đen trắng) (`apps/web/src/features/next/lib/mau.ts`), bỏ Tinh vân / Cực quang / Bình minh sương / Console.
- 2026-11-20 v107: chat điện thoại (`apps/shell/src/remote/chat.rs`), menu bubble trên điện thoại.
- 2026-11-17 v104: E2E mặc định Tauri, turn changes, clash chip, inbox, deepseek lazy.
- 2026-11-15 v102: UI next gen "Sương kính" (`apps/web/src/features/next/components/NextShell.vue`), mặc định từ v103.
- 2026-11-15 v101: Disk manager (`apps/core/crates/core-disk/src/lib.rs`, `apps/shell/src/disk_map.rs`), MCP `disk_*`.
- 2026-11-23: terminal không refit khi cửa sổ minimize (`apps/web/src/features/workspace/lib/splitLayout.ts::viewportFits`).
- 2026-11-23: Sương kính bỏ blur, luôn panel đặc (`apps/web/src/features/next/stores/uiGen.ts`).
- 2026-11-15 v100: click menu Claude / Codex bằng chuột (`apps/web/src/features/click-menu/lib/termMenu.ts`).
- 2026-11-12 v97 / v98: Orrery Remote (app Flutter `apps/mobile`, PWA `apps/pwa`, relay `apps/relay`, gateway `apps/shell/src/remote/mod.rs`), push FCM / Web Push.
- 2026-11-10 v87: Rust thay Node collector + PTY + procs + usage (`apps/core/crates/core-serve/src/lib.rs`).
