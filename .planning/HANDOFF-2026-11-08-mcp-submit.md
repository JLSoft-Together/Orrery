# HANDOFF — open_session `submit: true` (owner exception 2026-11-08)

## Mục tiêu

Chủ nhân duyệt (2026-11-08): agent trong pane Orrery được mở session mới **và tự Enter prompt** qua MCP
`open_session` với `submit: true`. Dùng cho jin-workflow: Claude hết quota → hook `quota-fallback` mở pane
Codex trong Orrery, chạy tiếp pipeline mà không cần chủ nhân bấm Enter (hiện jin phải chạy `codex exec` ngầm).

## Đã sửa (working tree, CHƯA commit, typecheck sạch, test cũ pass)

- `packages/shared/src/pty.ts` — `PaneOpenRequest.submit?: true`.
- `packages/shared/src/orreryApi.ts` — `OpenPolicy.askSubmit` (mcp `true`, còn lại `false`); `parsePaneOpen` nhận
  `submit` boolean (khác kiểu → null), chỉ giữ khi có `from` (pane gọi) + prompt + preset khác `shell`; link
  `agent-monitor://open` (không `from`) không bao giờ submit.
- `apps/web/src/stores/workspace.ts` `openSession` — `prefillSubmit` khi `policy.submit ? input.submit !== false : policy.askSubmit && input.submit === true`.
- `apps/desktop/src/pty/mcpTools/types.ts` — `OpenArgs.submit`, `OpenOutcome.submitted`.
- `apps/desktop/src/pty/host.ts` `openFrom` — `submit = args.submit === true && !!prompt`, chuyển xuống `askPage`, outcome `submitted`.
- `apps/desktop/src/pty/mcpTools/openSession.ts` — schema `submit` (boolean, default false, mô tả chỉ dùng khi
  chủ nhân đã cho làm tiếp không cần người), mô tả `prompt` sửa "NOT submitted unless submit is true",
  `openText` báo "sent as soon as the agent is ready" khi submitted.

Giới hạn sẵn có vẫn áp: `OPEN_RATE_MAX` / phút / pane, `PANE_LIVE_MAX` 16, `TermPane` chờ agent ready mới Enter
(dùng chung đường `prefillSubmit` của Telegram).

## Còn làm

1. Test:
   - `apps/web/src/stores/workspace.test.ts` (gần dòng 454): `openFromAgent({ from, prompt, submit: true })` →
     pane có `prefillSubmit: true`; không `submit` → không; `from` vắng (url) + `submit` → không.
   - Test parse `session.open` (`useOrreryApi.test.ts` hoặc `orrery-api.test.ts`): `submit: 'yes'` → `args`;
     `submit: true` không `from` / không prompt / preset shell → bị bỏ.
   - `apps/desktop/test/mcp.test.ts` (~dòng 68): `openText` có `submitted`.
2. Docs:
   - `CLAUDE.md` mục Core principle: thêm **ngoại lệ thứ 4** (v-tiếp theo): MCP `open_session` `submit: true` từ
     pane agent, owner 2026-11-08, lý do jin-workflow chuyển Codex khi hết quota.
   - Đoạn "Since v49 … prompt only prefilled, never submitted" → thêm "unless the agent passes submit: true (owner 2026-11-08)".
   - `docs/claude-code-integration.md` §11w (open_session) + `.code_index/protocol.md` (orreryApi OPEN_POLICY
     askSubmit), `.code_index/desktop.md` (host openFrom submit), `.code_index/_meta.md` 1 dòng.
3. `npm run typecheck` + test các file trên. Không build verify. Không commit nếu chủ nhân chưa bảo.

## Sau khi xong

Báo session jin (pane `dauph`, project `C:\Users\dauph`) hoặc chủ nhân: jin sẽ đổi `quota-fallback` gọi
`open_session { agent: 'codex', cwd, prompt, submit: true }` khi đang trong pane Orrery.
