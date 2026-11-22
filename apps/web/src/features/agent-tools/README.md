# agent-tools

Công cụ quanh agent: prompt library + queue + attachments, turn review / cards, clash + overlap chip, handoff / compact, permission advisor, plan board, notes, context meter / sources, limit resume, session tools sheet, bộ lọc session.

- Đường vào: `@features/agent-tools` (`useCompact`, `usePromptsStore`, `useTurnEditsStore`, `useAgentTouchesStore`, `useAttachmentsStore`, `SessionFilterBar`, `hasPermAdvisor`, `hasCtxSources`...). Chỉ export phần feature khác cần.
- `components/workspace/*` (WorkspaceView, TermPane...) còn import sâu `features/agent-tools/...`.
- File chính: `stores/turnEdits.ts`, `stores/agentTouches.ts`, `components/PromptPicker.vue`, `components/TurnReview.vue`, `composables/usePaneAttachments.ts`.
- i18n: `i18n/{context-v72,perm-v72,resume-v73,session-tools-v89}.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
