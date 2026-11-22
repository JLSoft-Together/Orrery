# PLAN v110: chia module, gom gọn, index rẻ token

Chủ nhân chốt 2026-11-22. Bối cảnh: TS chiếm 56%, đang chuyển dần sang Rust; cần chia module, gom gọn, index hợp lý để dễ maintain và agent đọc ít token.

## Số đo ban đầu (dòng code, bỏ test / generated)
| Phần | Ngôn ngữ | Dòng |
|---|---|---|
| apps/web | ts + vue | 70 150 + 66 889 |
| apps/desktop (Electron) | ts | 54 999 |
| packages/habblaud (Office) | ts | 43 694 |
| apps/collector (Node) | ts | 14 680 |
| packages/shared | ts | 7 750 |
| apps/pwa | ts + vue | 7 973 + 12 256 |
| apps/core | rs | 125 250 |
| apps/shell | rs | 45 176 |
| apps/mobile | dart + rs | 33 475 + 1 543 |
Index `.code_index/` 696 KB; `CLAUDE.md` 26 KB nạp mỗi phiên.

## Quyết định
1. **Bỏ hẳn Electron + collector Node.** Chuyển `apps/desktop/src/preload.ts` (nguồn shim `scripts/gen-desk-shim.mjs`) và `apps/desktop/daemon-kit.mjs` (dùng bởi `scripts/dist-tauri.mjs`) sang chỗ mới; xoá Electron main, collector, E2E chỉ của Electron, script `dist:electron*`, env `CCM_CORE*=node`. Trước khi xoá: bảng parity, phần nào Rust chưa có phải làm hoặc chủ nhân duyệt bỏ.
2. **Office tách module riêng, tải khi bật** (lazy import, index riêng, agent không đọc khi làm việc khác). Office vẫn tạm dừng phát triển.
3. **apps/web chia theo tính năng**: `src/features/<feature>/{components,lib,stores,composables,i18n}` + `src/core` dùng chung (ws client, theme, ui kit, i18n gốc). Mỗi feature có `README.md` ngắn (đường vào, file chính). Chuyển từng đợt, mỗi đợt qua `npm run qa` + E2E map.
4. **Index cây module + lookup**: `.code_index/<module>.md` max ~8 KB (đường vào, file chính, bất biến), `lookup.md` việc → `path::Symbol`, lịch sử v-số sang `.code_index/archive/`. `CLAUDE.md` gọn còn ~6 KB, lịch sử tính năng sang `docs/`.

## Pha
- [ ] P0 khảo sát (chỉ đọc, chạy song song với v109): `.planning/AUDIT-v110-parity.md` (Electron main + collector → Rust đã có / thiếu), `.planning/AUDIT-v110-web-features.md` (map file → feature, phụ thuộc chéo, file > 800 dòng), đề xuất cây index.
- [x] P0 xong: `AUDIT-v110-parity.md`, `AUDIT-v110-web-features.md` (23 feature + core, 13 đợt), `AUDIT-v110-index.md` (696 KB → ~200 KB, 40 file ≤ 8 KB).
- [x] Chủ nhân duyệt P0 (2026-11-22): LÀM bằng Rust trước khi xoá Electron cả 4: (1) mở / focus ngoài app (`terminal.focusWindow`, `terminal.open`, `favorite.open`, `folder.pick`, `core-monitor/src/monitor/deps.rs` `SystemDeps`), (2) macOS PATH login shell + menu app + Move to Applications, (3) shell marks OSC 633 (gộp với v109 F6 / B3), (4) macOS Dock bounce + toast có nút. Office server (chỉ chạy với collector Node): bỏ cùng collector, Office tách module theo P2.
- [x] P0.5 parity Rust (4 mục trên): `core-monitor/src/native/` (`NativeDeps`), shell `folder_pick.rs`, `mac_env.rs`, `mac_menu.rs`, `mac_move.rs`, `attention.rs`, `mac_note.rs`; OSC 633 trong v109 F6. E2E `v110-external-open.cjs` 5/5. macOS CHƯA compile / chạy thử (máy Windows không có target `aarch64-apple-darwin`): cần chạy trên Mac trước khi xoá Electron.
- [x] P1 bỏ Electron + collector: xoá `apps/desktop` (519 file) + `apps/collector` (144 file); E2E sang `apps/shell/e2e/`, version `apps/shell/package.json`, shim `apps/shell/shim/preload.ts`, `npm run dev` = `orrery-core serve --dev` + Vite.
- [x] P2 Office tách `apps/web/src/features/office/` lazy (70 file), event qua `core/stores/connectionEvents.ts`, i18n Office nạp khi mở; chunk chính 2217 -> 2184 kB.
- [x] P3 web theo feature (lô A-12): `apps/web/src` chia `core/` + `app/` + `features/<f>/` (alias `@core`, `@features`), luật `importRules.test.ts`, codemod `scripts/web-move.mjs`; `src/{components,lib,composables,stores}` đã xoá. Còn feature -> app 44 cạnh và import sâu giữa feature, E2E chờ máy rảnh.
- [x] P4 index mới: `.code_index/` 38 file tối đa 8 KB (lookup + module), `CLAUDE.md` 26 KB -> 7.3 KB, `docs/{features-history,core-principle,env,visual-style}.md`, `scripts/index-check.mjs` (0 đường chết); log E2E `shell.log`. Chưa làm: cập nhật skill indexer ngoài repo nếu nó đọc tên file cũ.
- [ ] P5 đo lại: dòng TS / Rust, KB index, token đọc 1 task mẫu.

## Chạy (2026-11-22, chủ nhân bảo làm luôn P1-P4)
- Chủ nhân ra lệnh làm P1 ngay, không chờ kiểm Mac; phần macOS P0.5 vẫn cần chạy thử trên Mac sau (Electron còn trong git history, `f5a208bc` là commit cuối có Electron).
- Main chốt thay chủ nhân (việc kỹ thuật, không đổi hành vi): `core-desk/src/mcp/tools.json` thành nguồn, bỏ `scripts/gen-mcp-tools.mjs` (cách 1); version app ở `apps/shell/package.json` (workspace `@ccm/shell`); E2E dời `apps/desktop/e2e` → `apps/shell/e2e`, bỏ E2E chỉ Electron; `npm run dev` = `orrery-core serve --dev` + Vite; Office server bỏ cùng collector, web Office hiện chữ "chưa có trên bản này" thay `OfficeNeedsNode`.
- Pipeline `.planning/pipeline.json` (`v110-modules`), bảng WP + tiến độ ở `.planning/PROGRESS.md`.

## Ràng buộc
- Không đổi hành vi app; dời file dùng `git mv` để giữ lịch sử.
- Không comment trong code; không commit / push khi chủ nhân chưa bảo.
- Pha chuyển file chỉ chạy khi không có gói khác sửa cùng vùng.

## P5 đo lại (2026-11-22, phần đếm dòng; gate full chờ máy rảnh)
| Phần | Trước | Sau |
|---|---|---|
| TS + Vue (web, desktop, collector, habblaud, shared, pwa; bỏ test) | khoảng 278 000 | khoảng 214 000 (web 142 136, pwa 20 369, habblaud 43 694, shared 7 753) |
| Rust (core + shell) | 170 426 | 174 139 (+ parity P0.5) |
| TS trong TS + Rust + Dart | khoảng 57 % | khoảng 50 % |
| Test TS bỏ cùng Electron + collector | | khoảng 51 000 dòng |
| `.code_index` đang dùng | 696 KB, 19 file | 88 KB, 38 file ≤ 8 KB |
| `CLAUDE.md` nạp mỗi phiên | 25.6 KB | 7.5 KB (bớt khoảng 4 500 token mỗi phiên) |
