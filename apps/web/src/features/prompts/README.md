# prompts

Prompt đã lưu (v114 G2): lưu trong file `prompts.json` của data dir (core-desk, desktop + phone dùng chung), chèn vào ô nhập của pane agent, KHÔNG bao giờ submit.

- Đường vào: `@features/prompts` (`PromptsSettings`, `useSavedPromptsStore`, `usePromptInsert`, `usePalettePrompts`, `usePromptsMenu`, `usePromptSave`).
- Kênh core: `ccm:prompts-get`, `ccm:prompts-set {op}`, event `ccm:prompts-changed` (`apps/core/crates/core-desk/src/prompts.rs`). Web chỉ đọc / ghi qua `lib/promptBridge.ts`, store `stores/savedPrompts.ts` (Pinia id `savedPrompts`, khác store `prompts` cũ của agent-tools).
- Chèn: `workspace.update(key, { prefill, prefillSubmit: undefined, prefillNow: true })` (`lib/insertPrompt.ts`), chặn khi pane có `prefill` / `compact`. Biến `{branch}` `{project}` `{cwd}` ở `lib/promptVars.ts`, thiếu giá trị thì giữ nguyên chữ và báo nhẹ.
- Palette: prefix `;` (provider `composables/usePalettePrompts.ts`), mục menu header: `composables/usePromptsMenu.ts`.
- Mục có chuỗi giống secret (`looksSecret`) hiện nhãn "Chỉ desktop"; phone không nhận (shell lọc bằng `mask_text`).
- i18n: `i18n/saved-prompts.ts`, import tĩnh trong `src/i18n/{en,vi}.ts`.
