# v97 — khảo sát phía renderer + core-pty (2026-11-12)

Subagent sonnet khảo sát, main lưu lại. `W` = `apps/web/src`, `C` = `apps/core/crates/core-pty/src`.

## 1. Workspace shape
- Pane `W/stores/workspace.ts:62-122`. Persisted (`persistNow` :562-604): `key, preset, cwd, sessionId, resume, agentSessionId, mintedSession, untouchedSession, ranCommand, command, openedBy, parked, kind/url/path/line/root`. Runtime: `state` (`parked|starting|running|exited|gone|error` :46), `pid, liveCwd, agent, terminalId, attention, branch, dirty, prefill, prefillSubmit, typeKeys, compact, title, count, select, exitCode, error, focusedAt`.
- Tab :124-134 `{key, root: LayoutNode, focus, project?, name?, folded?, zoom?}`.
- Split tree `W/lib/splitLayout.ts:3` `{pane} | {dir:'row'|'col', ratio, a, b}`; `leaves` :28, `boxes`.
- Pane → PTY = `pane.sessionId` (từ `ready`, `TermPane.vue:828`). Pane → `TerminalSession` = `pane.terminalId` (`useWorkspaceLinks.ts:44-57` qua `owners`). Ngược: `workspace.paneByTerminal` :553.
- Summary đề xuất (~1–3 KB): `{rev, activeTab, activePane, tabs:[{key,name,project,zoom,focus,folded,tree, panes:[{key,kind,preset,agent,title,cwd,branch,state,rail,waitText,attention,subagents,sessionId,terminalId,dirty,busy}]}], closedCount, pendingClose?}`. Dựng từ `lib/rail/mission.ts` `missionGrid` hoặc tabs + leaves.

## 2. Action
- Bảng action có sẵn: `packages/shared/src/orreryApi.ts` `ORRERY_ACTIONS` :243-274, `OrrerySource` :8, `OPEN_POLICY` :40-48; `W/composables/useOrreryApi.ts` `handleOrrery` / `runOrrery` :22-41. Telegram đăng ký `session.status|send|sendAll|keys|tail|compact|closeOwner|focus|create|resumeList|resume|answer` (`useTelegramInbound.ts:201-212`). Phone = thêm source `'phone'` + hàng `OPEN_POLICY`.
- `openSession` :840-877 → `{ok,paneKey}` | `{ok:false,error}`; limit `PANE_LIVE_MAX` 16; `resume` đang chạy → activate; live chỗ khác → `live`.
- Close: `requestClose(keys,busy,{record})` :1243 → bận thì `pendingClose` (ref runtime); `confirmClose` :1254 / `cancelClose` :1262. Sheet = Vue in-page `CloseConfirmDialog.vue` (mount `WorkspaceView.vue:1966-1976`). `closePanes` (`view/useCloseFlow.ts:67`) lưu file dirty trước; `confirmPending` :256 là đường user. `isBusy` `usePaneInfo.ts:176`. Telegram close: `requestClose([key], () => false)` + check `since` (`useTelegramInbound.ts:140`).
- `renameTab` :1491, `movePaneToTab` :1519, `moveTab`, `setFolded`, `ungroup` = chỉ state. `reopenClosed` :1329 (`CLOSED_MAX` 10). `split` :1055 và `toggleZoom` :1365 đổi focus desktop. Compact `useCompact().start` (`useCompact.ts:33`).
- Không người ngồi máy vẫn an toàn: rename/move/fold, `status`, `tail` (cần xterm mounted), `send`/`answer` (prefill chờ idle), `keys` (`keysReady` + `since`), `resume`, `create`. Cần quyết: focus/split/zoom (cướp focus desktop), close/reopen.
- Renderer phải sống + Workspace mounted; prefill do `TermPane` gõ (`prefillReady` `TermPane.vue:524`, fallback 20 s).

## 3. Điểm phát summary
- `workspace.ts:606-613` deep watch `[panes,tabs,activeKey]` → `PERSIST_MS` 400 (:237). Thêm timer 200 ms riêng. Status `working/idle/waiting` nằm ở `terminals` store → watch thêm (`useWorkspaceLinks.ts:107-128` đã có diff watcher). Composable mới `useRemoteSummary`, so hash JSON trước khi gửi.

## 4. Terminal + core-pty
- `W/lib/ptyClient.ts` `PtyConnection.open`: 1 WS / pane, `{t:'spawn',…,cols,rows}` | `{t:'attach',id}` (:98-101) → `ready`, `cwd`, 1 frame replay, `branch`, `exit`. Input binary, `{t:'resize'}` từ `term.onResize` (`TermPane.vue:1023`). Close khác 4001 → pane `error` "disconnected" (:881-886).
- Hub: `Session.conn: Option<u64>` (`C/hub/mod.rs:63`) = 1 socket / session. `attach` (`spawn.rs:88-136`) đóng socket cũ `4000 replaced` (:129). Output chỉ tới `s.conn` (`from_main.rs:136`). Resize last-writer-wins (`frames.rs:76`). Meter / Gate 1 conn (`flow.rs:104`, bind :133-135). `Closed` (`mod.rs:229-238`) và reaper `s.conn.is_none()` (`from_main.rs:117`) giả định 1 conn.
- ⇒ **Phải sửa hub**: `viewers: Vec<conn>` + `input_owner`; fanout data / exit tới mọi viewer, ring ghi 1 lần, replay mỗi attach; bỏ `4000` cho viewer; viewer chậm không được chặn PTY (meter riêng, chỉ meter của owner bind Gate, viewer tràn → close 4001 + attach lại); message `attach {mode:'view'}`, `{t:'input-claim'}`, resize chỉ nhận từ owner, viewer nhận `resized {cols,rows}`. Test `C/hub/tests.rs`, `core-bin/tests/pty.rs`.

## 5. Attention / inbox
- `Pane.attention` (`waiting|done`) latch ở `useWorkspaceLinks.ts:104-128`. `paneState` `W/lib/rail/railModel.ts:137` → `parked|starting|failed|asked|ended|working|subagents|done|idle`. `workspace.attentionPanes` :554, `nextAttention` :1233.
- Inbox `stores/inbox.ts` + `lib/inbox.ts` `inboxItems()` (waiting|note|loop|checks|stuck|done, dismiss `ccm.inbox`) → dùng lại cho inbox điện thoại. `TelegramPaneInfo` (`useSessionActions.paneInfo` :61) = dòng phone. Nút xin quyền = `session.keys` + `since`.

## 6. Mission Control
- `MissionControl.vue` (1070 dòng) + `mission/MissionCard.vue`; model `lib/rail/mission.ts:28` card `{tabKey,title,named,current,focus,state,subagents,needs,failed,tiles[]}`, section theo project. Tail lines từ `paneSnapshots` (xterm buffer).

## 7. Submit vào pane idle
- `workspace.sendPrompt` :967-974 (`prefill` + `prefillSubmit`). `TermPane.vue` `prefillReady` :524-527, `sendPrefill` :532-552 (slash raw, text `term.paste`, `\r` sau 200 ms). `answer` (`useTelegramInbound.ts:161-173`) = mẫu "idle thì submit, không thì chỉ gõ". `typeKeys` :976 cho phím thô.

## 8. IME
- `W/lib/termIme.ts` = vá Telex trên WKWebView. Điện thoại không đi qua textarea xterm ⇒ gửi cả dòng (`session.send`) hoặc phím đơn (`typeKeys`), không stream từng ký tự sửa. `TEXT_MAX` 20 000.
