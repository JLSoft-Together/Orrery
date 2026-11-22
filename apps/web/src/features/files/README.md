# files

Explorer (tree, search, TODO, checks, changes tab, quick open) and the file viewer / editor (FilePane, CodeMirror 6, minimap, outline, JSON tree, HTTP panel, markdown, syntax highlight worker).

- Đường vào: `@features/files` (`AccessPrompt`, `useChecksStore`, `joinPath`, `renderMarkdown`, `tokenize`, `editorTheme`, `explorerMenu`, `dragPaths`, `AUTO_SAVE_*`...). Chỉ export phần feature khác cần.
- App nạp thẳng (không qua index để giữ chunk lazy): `FilePane`, `ExplorerPanel`, `QuickOpen`, `NavPicker`, `CodeEditor` trong `components/workspace/*`.
- File chính: `components/FilePane.vue`, `components/ExplorerTree.vue`, `stores/explorer.ts`, `lib/fileEditor.ts`.
- `lib/explorer/fileIconRules.ts` do `scripts/gen-file-icons.mjs` sinh (giữ nguyên).
- `lib/highlight.worker.ts` import trực tiếp `../../git/lib/diffBlocks` (worker không được kéo cả index git).
- i18n: `i18n/*.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
