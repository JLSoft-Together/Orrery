# web-agent-tools: công cụ quanh agent

Đường vào: `apps/web/src/features/agent-tools/index.ts` (`useCompact`, `usePromptsStore`, `useTurnEditsStore`, `useAgentTouchesStore`, `useAttachmentsStore`, `SessionFilterBar`, `hasPermAdvisor`, `hasCtxSources`). `components/workspace/*` còn import sâu `features/agent-tools/...`.

Nội dung: prompt library + queue + attachments, turn review / cards, clash + overlap chip, handoff / compact, permission advisor, plan board, notes, context meter / sources, limit resume, session tools sheet, bộ lọc session.

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/agent-tools/stores/turnEdits.ts` | file agent sửa theo turn (turn review, hoàn tác) |
| `apps/web/src/features/agent-tools/stores/agentTouches.ts` | agent nào chạm file nào (clash / overlap) |
| `apps/web/src/features/agent-tools/components/PromptPicker.vue` | thư viện prompt |
| `apps/web/src/features/agent-tools/components/TurnReview.vue` | xem lại thay đổi theo turn |
| `apps/web/src/features/agent-tools/composables/usePaneAttachments.ts` | đính kèm vào prompt |

## Bất biến

- Compact (v51): chỉ submit 1 prompt handoff vào pane idle chủ nhân chọn; `/compact` và dòng resume tự gõ khi switch bật, dừng nếu gõ tay / huỷ / chờ 10 phút (docs §11x, `docs/core-principle.md`).
- `hand_back` từ DeepSeek submit đúng 1 prompt cố định (docs §11am).
- Không gửi prompt từ monitor; chỉ hành động do chủ nhân bấm.

## Liên kết

- i18n: `apps/web/src/features/agent-tools/i18n/`. Plan: `.planning/PLAN-v51-*.md`, `PLAN-v72-*.md`, `PLAN-v73-*.md`. E2E: `v51-compact`, `v89-session-tools`, `v104-turn-changes`, `v104-clash`.
