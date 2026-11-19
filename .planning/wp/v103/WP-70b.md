## WP-70b: UI next — menu góc phải tắt khi click ra ngoài, chế độ gọn về lại đầy đủ, bỏ Mission Control, UI next mặc định

- Plan + luật chung: `.planning/PLAN-v103-next-polish.md` (mục "Luật chung" + "Quyết định main", bắt buộc). Góp ý #2, #4, #12 + "bản mới mặc định UI next".
- Đọc trước: `CLAUDE.md` (v102, v100 click menu: trả lời từ Mission Control card), `.code_index/web.md`, `apps/web/src/components/next/**`, `apps/web/src/lib/uiGeneration.ts`, `apps/web/src/stores/uiGen.ts`, `apps/web/index.html` (boot `pick`), `lib/themes.ts` `startTheme` (mẫu migration `4.0.2-galaxy`), `stores/ui.ts` (`compact`), `components/CompactView.vue`, Mission Control (`components/workspace/**`, rg `missionControl|MissionControl`). WP-70a vừa xong: `.planning/wp/v103/WP-70a-report.md` (NextShell `baseMode` / `collapsed`, `NextQuotaBar`, `--ccm-header-h` next 50px — đừng làm hỏng, E2E `apps/desktop/e2e/v103-rail.cjs` phải còn pass).
- Model: sonnet. Song song với WP-72 (git viewer) và WP-71 (Explorer bên phải) — không đụng file của họ.

### Việc
1. #2 Menu chức năng góc phải (nút "…" / menu trên thanh tab hoặc header next — xác định đúng bằng ảnh chụp) không tắt khi click ra ngoài → tắt khi click ra ngoài, Esc, mất focus cửa sổ; kiểm mọi menu / popover của next cùng kiểu (dùng chung 1 cơ chế, không vá từng cái).
2. #4 Vào chế độ gọn (compact) rồi không về được chế độ đầy đủ → có đường về rõ ràng (nút trong compact view, phím tắt, menu tray / Ctrl+K), chạy cả Tauri lẫn Electron; tìm nguyên nhân gốc (state `ui.compact` / kích thước cửa sổ / IPC).
3. #12 Bỏ Mission Control: gỡ lệnh, phím tắt, mục Ctrl+K, nút, card grid, phím 1–9, i18n, test / E2E liên quan (classic cũng bỏ, vì sẽ bỏ classic). Chức năng trả lời menu v100 từ Mission Control: đảm bảo vẫn trả lời được qua chip "Chờ trả lời" (Ctrl+Alt+Q) + toast + thanh dưới pane. Cập nhật `CLAUDE.md` (câu v50 / v52 / v100 nhắc Mission Control) và docs liên quan.
4. UI next mặc định: cài mới → next; máy đang có `ccm.uiGeneration = classic` → migration 1 lần sang next (khoá migration riêng, vd `ccm.uiMigration = 4.1-next`), không ép lại lần sau nếu chủ nhân chọn classic; `index.html` boot khớp để không nháy UI cũ; Appearance vẫn có lựa chọn classic (ghi chú "sẽ bỏ"). Test cho migration.
5. Test vitest + E2E next (menu click outside, compact ↔ full, không còn Mission Control, migration), ảnh `docs/qa/v103/WP-70b/`. Báo cáo có mục "Test macOS".

### Báo cáo
`.planning/wp/v103/WP-70b-report.md`.
