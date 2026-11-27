## WP-114f — PARTIAL (code + test xong; chờ main nối i18n + 3 dòng TermPane; chưa nhìn UI)

### Đã đổi
Core (Rust, `apps/core/crates/core-desk/src/`):
- `prompts.rs` (mới, 305 dòng) + `prompts_tests.rs` (mới): kênh `ccm:prompts-get`, `ccm:prompts-set`, event `ccm:prompts-changed`. File `<data dir>/prompts.json`, ghi nguyên tử (`util::write_atomic`) dưới 1 Mutex, max 200 mục, title <= 80, body <= 4000 (đếm UTF-16 như `.length` của web), từ chối thay vì cắt. File hỏng -> đổi tên `prompts.json.bak`, bắt đầu rỗng, trả `recovered:true`. File `version` > 1 hoặc không đọc được -> `{ok:false,error:'io'}`, không bao giờ ghi đè.
- `lib.rs`: `pub mod prompts;` + `prompts::register(...)` trong `Desk::standard`.

Shell / bridge:
- `apps/shell/shim/preload.ts`: `promptsGet`, `promptsSet`, `onPromptsChanged`; `apps/shell/src/shim.js` sinh lại bằng `node scripts/gen-desk-shim.mjs` (`--check` báo up to date).
- `apps/web/src/core/lib/desktop.ts`: thêm 3 type vào `DesktopBridge`.

Web:
- `apps/web/src/features/prompts/**` (mới): `lib/{promptModel,promptVars,promptBridge,insertPrompt,palettePrompts,paletteSeed,legacyPrompts}.ts`, `stores/savedPrompts.ts` (Pinia id `savedPrompts`), `composables/{useInsertPrompt,usePalettePrompts,usePromptsMenu,useSavePrompt}.ts`, `components/{PromptsSettings,PromptEditor}.vue`, `i18n/saved-prompts.ts`, `index.ts`, `README.md`, test cho từng lớp.
- `features/next/lib/palette.ts`: `;` -> mode `prompts` (`NextPrefixMode`, `NextScope`, `NEXT_PREFIX`, `NEXT_MODES`, `NextGroup`, `NEXT_GROUP_ORDER`, `inNextScope`); `NEXT_SCOPES` KHÔNG đổi (không có tab scope mới). Test thêm vào `palette.test.ts`.
- `features/palette/components/CommandPalette.vue` (+4 dòng, brief cho 1-3): import, `Scope` thêm `'prompts'`, 1 dòng `usePalettePrompts(...)`, 1 dòng thêm `promptItems` vào `matched`.
- `features/settings/components/PageWorkspace.vue` (+2 dòng) và `components/index.ts` (+1 entry `savedPrompts`, tìm được qua Settings search).
- `features/history/components/PromptSearchCard.vue`: nút "Lưu thành prompt" trên từng snippet kết quả tìm prompt cũ.
- `features/workspace/lib/paneHeadActions.ts`: `menuKeys` thêm `'prompts'` ở cuối cho pane agent (kiểu `MenuKey`), test cập nhật.
- `apps/shell/e2e/v114-prompts.cjs` (mới, CHƯA KIỂM).

### Phát hiện lệch spec (quan trọng)
Spec ghi "Không có thư viện prompt". SAI: đã có thư viện cũ `features/agent-tools` (store `prompts`, localStorage `ccm.prompts`, `PromptPicker.vue`, samples, import / export `ccm:prompts-export|import`, biến `{file}{selection}{branch}{diffstat}{checks}{wt}`). Cách xử lý trong gói: giữ nguyên agent-tools (ngoài phạm vi), làm kho mới theo D4, migrate 1 lần từ `ccm.prompts` sang core khi core rỗng (cờ `ccm.prompts.coreMigrated`; scope dự án cũ là `dirKey` viết thường, chỉ giữ nếu còn là đường dẫn tuyệt đối, không thì thành "Mọi dự án"). Hệ quả: `PromptPicker` cũ vẫn ghi localStorage riêng, 2 nguồn lệch nhau cho tới khi main quyết định (khuyến nghị: nối `agent-tools/stores/prompts.ts` vào `useSavedPromptsStore` rồi bỏ localStorage, hoặc gỡ PromptPicker). Import / export Settings dùng lại kênh dialog sẵn có (`promptsExport` / `promptsImport`, file có khoá `prompts`), nhận cả khoá `items`.

### Tiêu chí
- core-desk đọc / ghi / giới hạn / file hỏng / ghi nguyên tử: ĐẠT (16 test).
- Biến `{branch}` `{project}` `{cwd}`, thiếu giá trị giữ nguyên chữ + báo nhẹ: ĐẠT.
- Chèn `prefill` + `prefillNow:true`, `prefillSubmit` undefined, chặn khi `prefill` / `compact`, nhiều dòng, dòng bắt đầu `/`: ĐẠT ở mức store (`workspace.update` thật). Hành vi gõ thật ở `TermPane.vue::sendPrefill` CHƯA KIỂM (xem Review).
- Palette `;`, Settings CRUD + xếp + nhập / xuất + undo xoá, "Lưu thành prompt": code xong, test logic; UI CHƯA NHÌN.
- Mục "Prompt đã lưu" trong menu header pane agent: CHƯA nối (cần sửa `TermPane.vue`, xem Ngoài phạm vi).
- Typecheck sạch; không comment; file mới đều < 400 dòng.

### Audit
- Diff trong phạm vi + các file nối đã liệt kê. Không đụng rail/*, railPeek, railSnooze, railLive, TermPane, remote, click-menu, shared/remote.ts, shell/src/remote/*, i18n/{en,vi}.ts, agent-tools.
- Không có git commit / stash / reset. `index-check.mjs` báo `CLAUDE.md 7.6 KB vượt 7.5 KB` (có từ trước, không thuộc gói). `.code_index` CHƯA cập nhật (ngoài phạm vi gói, main ghi 1 dòng `_meta.md`).

### Review
- File hỏng: đổi tên `.bak` (kể cả `.bak` cũ bị ghi đè) + `recovered`, web hiện cảnh báo. File phiên bản mới hơn / lỗi đọc: từ chối ghi (không mất dữ liệu).
- Ghi đồng thời: Mutex trong `register` (test 8 thread x 5 ghi giữ đủ 40). `util::write_atomic` đặt tmp theo pid nên LUẬT: 114g (shell phone ops) phải gọi qua `desk.dispatch("ccm:prompts-set", ...)` chứ không tự ghi file, nếu không mất khoá.
- Web: mỗi request gắn số thứ tự, trả lời cũ không đè trả lời mới (test).
- Prompt nhiều dòng: giữ nguyên `\n` (CRLF -> LF). `TermPane.sendPrefill` chỉ giữ xuống dòng khi `term.modes.bracketedPasteMode`, nếu không thì gộp 1 dòng: hành vi đó của TermPane, ngoài phạm vi, CHƯA KIỂM trên Claude / Codex thật.
- Dòng bắt đầu `/` một dòng: `conn.write(text)` không kèm Enter, vẫn chỉ chèn (đọc code `TermPane.vue:768`, CHƯA KIỂM chạy thật).
- Pane đang bận: `prefillReady()` cho `working` khi `prefillNow`; pane `waiting` (đang hỏi menu) thì `prefill` đợi tới khi hết `waiting`, UI báo "Agent đang chờ câu trả lời, prompt sẽ được gõ khi xong". Không bao giờ gõ vào menu.
- Biến thiếu: giữ `{branch}`, toast liệt kê biến thiếu, vẫn chèn.
- D11: web chỉ đánh nhãn "Chỉ desktop" bằng `maskSecrets(body,'','linux') !== body`; lọc thật khi gửi phone là việc 114g (`mask_text` Rust có thể lệch quy tắc với `maskSecrets`, 114g cần test mục sát biên).
- Pane đã đóng giữa lúc chọn: `usePromptInsert.insert` đọc pane tại thời điểm chạy, không có pane -> `noPane`.
- Giới hạn còn lại: undo xoá đưa mục về CUỐI danh sách (upsert id cũ), không về vị trí cũ.

### Test
- `cargo test --locked -j 4 -p core-desk` (apps/core) -> 449 passed, 0 failed (16 test của prompts).
- `cargo clippy --locked -j 4 -p core-desk --all-targets` -> không cảnh báo nào ở prompts*.rs.
- `npm test -w @ccm/web -- src/features/prompts` -> 8 file, 48 test pass (model, vars, insert plan, palette rows, legacy, store, composable thật với workspace store, i18n parity).
- `npm test -w @ccm/web -- src/features/prompts src/features/next/lib src/features/palette src/features/history src/features/workspace/lib/paneHeadActions src/importRules` -> 35 file, 275 test pass.
- `npm test -w @ccm/web -- src/features/settings src/features/palette src/features/history src/features/workspace/lib src/importRules src/i18n src/features/next` -> 5 FAIL, đều do i18n chưa nối: `keys.snapshot` en / vi (snapshot, main chạy `-u` sau khi nối; trong diff còn có `v113Devices.toast.updateFail` của gói khác) và `settings/components/index.test.ts` 3 test "label / hint key resolves" (`savedPrompts.title`, `savedPrompts.hint` chưa có trong en / vi). Sẽ xanh khi nối i18n.
- `npm run typecheck` -> sạch.
- `node scripts/gen-desk-shim.mjs --check` -> shim up to date.
- E2E: KHÔNG chạy. `apps/shell/e2e/v114-prompts.cjs` viết sẵn (get / set / event, `;` liệt kê, chọn khi pane shell -> không gõ + toast): CHƯA KIỂM. E2E "chèn không gửi vào pane agent thật": CHƯA KIỂM, cần Claude / Codex thật hoặc fake agent.

### Hợp đồng kênh (để WP-114g dùng)
Kênh `invoke` (core-desk, qua `desk.dispatch` hoặc `ccm:*`):
- `ccm:prompts-get` (không tham số) -> `{ok:true, version:1, items:[{id,title,body,scope,updatedAt}], recovered:bool}` | `{ok:false, error:'io'}`.
- `ccm:prompts-set` (1 tham số object `{op, ...}`) -> `{ok:true, version:1, items:[...]}` (+ thêm trường theo op) | `{ok:false, error:<code>}`:
  - `{op:'upsert', item:{id?, title, body, scope?}}` -> thêm `id`. Có `id` đã tồn tại: sửa tại chỗ, giữ vị trí; `id` mới hợp lệ (`[A-Za-z0-9_-]{1,64}`): thêm cuối với id đó; không `id`: sinh uuid. `scope` mặc định `'all'`, hoặc đường dẫn tuyệt đối <= 4096.
  - `{op:'delete', id}` -> `not-found` nếu không có.
  - `{op:'reorder', ids:[...]}` -> id lạ / trùng bị bỏ, mục không nêu giữ thứ tự cũ, đặt cuối.
  - `{op:'import', items:[{title,body,scope?}]}` -> thêm `added`, `skipped`; luôn cấp id mới; hết chỗ thì `skipped`.
- Mã lỗi: `invalid-op`, `invalid-id`, `invalid-title`, `invalid-body`, `invalid-scope`, `too-many`, `not-found`, `io`.
- Event: `ccm:prompts-changed` payload `{count}` (chỉ khi ghi thành công; client phải `prompts-get` lại). `Desk::emit` đi tới webview qua `emit_app`; shell cần subscribe sink nếu muốn đẩy xuống phone (hiện `emit_app` chỉ phát cho MAIN / COMPACT webview, không có hook phone: 114g tự thêm giống nhánh `ccm:run-event` trong `apps/shell/src/desk.rs::emit_app`).
- Giới hạn (đồng bộ với web): max 200 mục, title 80, body 4000 UTF-16 units. Body đã chuẩn hoá `\n`.
- Hàm Rust dùng lại được: `core_desk::prompts::{load, set, Library, Item, file_for, CHANGED}`; `set(dir, &lock, &req, ctx)` cần đúng Mutex của `register` nên 114g nên đi qua `Caller` / `desk.dispatch`.
- Web bridge: `window.ccmDesktop.promptsGet()`, `.promptsSet(req)`, `.onPromptsChanged(cb) -> unsubscribe`.
- Mục "chỉ desktop" (D11): lọc ở shell khi trả `prompts.list`; web không gắn cờ vào file.

### Nhìn
CHƯA NHÌN: Settings "Prompt đã lưu" (hàng, editor, undo toast, trạng thái rỗng / lỗi), palette `;` (chip "Prompts", nhóm, mục "Quản lý"), nút "Lưu thành prompt" trong History (hover reveal), motion `ccm-list` 220 ms / reduced-motion. Cần mở app, 2 theme, en + vi.

### Hot file cần thêm
1. `apps/web/src/i18n/en.ts` và `vi.ts`: thêm `import savedPromptsV114 from '../features/prompts/i18n/saved-prompts'` cạnh dòng 127 và `...savedPromptsV114.en` / `...savedPromptsV114.vi` trong object spread (cạnh `...termHistoryV112.en` dòng 151). Rồi chạy `-u` snapshot `keys.*.txt`.
2. `apps/web/src/features/next/i18n/mist-v102.ts`: `mist.cmdk.prefix.prompts` = `'Prompts'` (en, cạnh dòng 142) / `'Prompt'` (vi, dòng 394). Thiếu khoá này thì chip `;` trong palette hiện tên khoá.
3. `apps/web/src/i18n/en.ts:1227` / `vi.ts:1228`: `palette.group.prompts` = `'Saved prompts'` / `'Prompt đã lưu'`. Thiếu thì tiêu đề nhóm trong palette hiện tên khoá.
4. `apps/web/src/features/workspace/components/TermPane.vue` (nối menu header, đang là vùng WP-114c): ba chỗ
   - `import { usePromptsMenu } from '@features/prompts'` và `const promptsMenu = usePromptsMenu()`;
   - trong `moreEntries` loop `menuKeys`: `else if (key === 'prompts') out.push(promptsMenu.entry.value)`;
   - đầu `pickMore(id)`: `if (promptsMenu.pick(id)) return`.
   `pick` mở palette rồi tự điền `;` (chạy sau khi palette tự xoá query).
5. `.code_index`: 1 dòng `_meta.md` + module `prompts` (ngoài phạm vi gói).

### Ngoài phạm vi
- `CommandPalette.vue` thêm 4 dòng thay vì 1-3 (import, type `Scope`, 1 composable, 1 mảng); không có cách ít hơn mà vẫn đúng kiểu.
- `agent-tools` (kho prompt cũ + PromptPicker) chưa đồng bộ với kho mới, xem "Phát hiện lệch spec".
- `TermPane.vue::sendPrefill` (gộp nhiều dòng khi không có bracketed paste; không gõ vào pane `waiting`) là hành vi có sẵn, không sửa.
- Phone: `prompts.list` / `prompts.save` / `prompts.delete`, event tới phone, cap `prompts`, lọc secret bằng `mask_text`, quota ghi: thuộc 114g / `apps/shell/src/remote/*`, không đụng.

### Commit message đề xuất
feat(prompts): shared saved-prompt library in core-desk with palette, settings and history entry points

- core-desk: prompts.json store behind ccm:prompts-get / ccm:prompts-set with atomic writes, limits, broken-file recovery and a changed event
- web: features/prompts (store, variables, insert without submit, ; palette mode, settings page with import / export / undo, save from prompt search), one-time migration from the legacy localStorage list
- shell: expose promptsGet / promptsSet / onPromptsChanged in the desk shim
