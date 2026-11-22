# web-files: Explorer, file viewer / editor

Đường vào: `apps/web/src/features/files/index.ts` (`AccessPrompt`, `useChecksStore`, `joinPath`, `renderMarkdown`, `tokenize`, `editorTheme`, `explorerMenu`, `dragPaths`, `AUTO_SAVE_*`). App nạp thẳng `FilePane`, `ExplorerPanel`, `QuickOpen`, `NavPicker`, `CodeEditor` (không qua index, giữ chunk lazy).

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/files/components/FilePane.vue` | pane xem / sửa file (CodeMirror 6, minimap, outline, JSON tree, HTTP panel, markdown) |
| `apps/web/src/features/files/components/ExplorerTree.vue` | cây Explorer, search, TODO, checks |
| `apps/web/src/features/files/stores/explorer.ts` | state Explorer (root, mở / đóng, chọn) |
| `apps/web/src/features/files/lib/fileEditor.ts` | đọc / ghi file, conflict đĩa, auto save |
| `apps/web/src/features/files/lib/highlight.worker.ts` | tô cú pháp trong worker (import trực tiếp `features/git/lib/diffBlocks`) |
| `apps/web/src/features/files/lib/explorer/fileIconRules.ts` | icon Atom Material (sinh bởi `scripts/gen-file-icons.mjs`, không sửa tay) |

## Bất biến

- Auto save mặc định tắt; không bao giờ ghi đè khi đĩa có thay đổi mới (conflict); bật thì đóng pane / thoát app lưu thay vì hỏi.
- Mọi đọc / ghi đi qua gate đường dẫn của core-desk (xem `core-desk-preview.md`); file thực thi không mở.
- files và git phụ thuộc vòng; chỉ file `.vue` đi qua index, lib thuần import sâu.

## Liên kết

- i18n: `apps/web/src/features/files/i18n/`. Plan: `.planning/PLAN-v59-file-icons.md`, `PLAN-v62-auto-save.md`. E2E: `explorer`, `file-editor`, `minimap`, `v103-explorer`.
