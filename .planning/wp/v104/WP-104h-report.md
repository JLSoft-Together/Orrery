# WP-104h - Changes theo session (báo cáo)

Changes pane có thanh "Theo session": Tất cả | mỗi session (StatusIcon + tên + số file) | "Không rõ". Tập file lượt cuối của mỗi session giữ tới khi "Đánh dấu đã xem lượt này" hoặc terminal đóng, không phụ thuộc `AFTER_TURN_MS` 2 phút hay `toolSpans` `KEEP_MS` 2 giờ. "Không rõ" luôn hiện khi có file không qua Edit / Write (Bash, formatter, sửa tay). Chỉ đọc `tool.span` (path + access) và git status, không gửi gì vào pane.

## File + symbol
| File | Thay đổi |
|---|---|
| `apps/web/src/lib/turnEdges.ts` (mới) | `TurnEdge`, `stepEdges`, `edgeSignature`: tách từ `stores/agentTouches.ts`. Khác 1 điểm: terminal thấy lần đầu ở idle có edge `{ working: false }`, nên lượt kế có `turnStart` thật thay vì fallback 10 phút |
| `apps/web/src/stores/agentTouches.ts` | dùng `stepEdges`, export `edges` (sửa tối thiểu, test cũ giữ nguyên) |
| `apps/web/src/lib/turnEdits.ts` (mới) | `TurnEdits`, `editPaths`, `addPaths`, `mergeTurn` (union, `turnStart` sớm nhất, max 2000 path), `splitBySession` (khớp path mới + `origPath` khi rename), `filterEntries`, `validFilter`, `UNKNOWN_FILTER`, `serializeTurns` / `parseTurns` |
| `apps/web/src/stores/turnEdits.ts` (mới) | `useTurnEditsStore`: `current` / `last`, watch `agentTouches.edges` (working mở lượt, hết working đóng lượt vào `last`, gộp với `last` chưa xem), watch `spans.version` thêm span đến muộn trong cửa sổ lượt, bỏ terminal `stale` / bị xoá (chỉ khi `connection.hasData`), `turns`, `countOf`, `markSeen`, lưu `sessionStorage` `ccm.turnEdits` (debounce 400 ms) |
| `apps/web/src/components/workspace/SessionFilterBar.vue` (mới) | segmented `[data-session-filter]` / `[data-session-seg]`, phím ←/→, container < 440px thành `AppSelect` `[data-session-select]`, slot cho nút đã xem |
| `apps/web/src/components/workspace/ChangesPane.vue` | prop `sessionFilter`, emit `filter`; `turnSplit`, `filter`, `shown` (nhóm, Chưa xem, F7 / [ ] đi theo danh sách đã lọc); badge Chưa xem của repo vẫn đếm toàn repo (`repoUnseen`); `markTurnSeen` `[data-turn-mark-seen]`; "Đánh dấu đã xem" khi đang lọc chỉ đánh các file đang hiện; `alsoBy` `[data-turn-also]`: icon `FileWarning` màu `warning` khi `settings.clashWarn` bật, không thì chữ nhỏ "cũng do {peer}" |
| `apps/web/src/stores/workspace.ts` | `Pane.sessionFilter` (runtime, không persist), `openChanges(cwd, { split, select, session })` |
| `apps/web/src/components/workspace/WorkspaceView.vue` | truyền `sessionFilter`, `@filter` → `workspace.update` |
| `apps/web/src/composables/useTurnChanges.ts` (mới) | `open(terminalId, cwd?)` (cwd từ pane hoặc terminal, mở Workspace), `has`, `cwdOf` |
| `apps/web/src/composables/useNotifications.ts` | `notify(..., onClick?)`; khi `done` và lượt có file: `Notification` bấm vào mở Changes đã lọc; toast trong app `[data-turn-toast]` "{name} xong, đổi {n} file" + nút "Xem thay đổi" `[data-turn-toast-view]` (cần `notifyEnabled` + `notifyDone`, tôn trọng giờ yên lặng + snooze), `turnToastId` |
| `apps/web/src/components/inbox/InboxPanel.vue` | nút "Xem thay đổi" có sẵn của nhóm Xong nối sang `useTurnChanges().open` (không thêm nút mới) |
| `apps/web/src/lib/rail/railMenu.ts` | `RowMenuCtx.turnChanges?: number`, mục `changesOfTurn` trước "Xem trong Giám sát", tắt + hint khi lượt không có file |
| `apps/web/src/components/next/rail/LiveRail.vue` | truyền `turnChanges`, `pickRow` `changesOfTurn` |
| `apps/web/src/components/CommandPalette.vue` | `c:turnChanges` cho pane đang active khi lượt có file |
| `apps/web/src/i18n/v104-ux.ts` | `v104.turnChanges.*` en + vi |
| `apps/desktop/e2e/v104-turn-changes.cjs` (mới), `run.cjs`, `scripts/qa-fast.mjs::E2E_MAP` | E2E Tauri + UI next, cổng 4531 / 9531, `CCM_DATA_DIR` tạm |
| `.code_index/web.md`, `.code_index/_meta.md` | mục WP-104h (web.md 44 952 byte) |

## Test
- `npm test -w @ccm/web -- src/lib/turnEdits.test.ts src/stores/turnEdits.test.ts src/lib/reviewQueue.test.ts src/lib/rail/railMenu.test.ts src/stores/agentTouches.test.ts src/lib/clash.test.ts src/lib/agentTouches.test.ts`: 7 file, 64 test pass. Toàn bộ web: 319 file, 2753 test pass (trước khi thêm ca "removed terminal", ca đó pass riêng).
- `npm run typecheck -w @ccm/web`: sạch.
- E2E Tauri debug, web build riêng vào scratch qua `CCM_WEB_DIST` (không ghi `apps/web/dist`):
  | File | Kết quả |
  |---|---|
  | `v104-turn-changes.cjs` (mới) | 23/23 PASS, chạy 3 lần |
  | `v104-clash.cjs` | 22/22 PASS, 2 lần |
  | `v104-inbox.cjs` (4511 / 9511) | 14/14 PASS |
  | `diff.cjs`, `split-diff.cjs` (4532 / 9532) | 30 / 29 PASS, 0 FAIL |
- Kịch bản E2E mới: 2 session (Alpha sửa `src/alpha.ts`, Beta sửa `src/beta.ts`) + `gen.txt` sửa ngoài span (Bash): lọc A chỉ ra `alpha.ts`; segment Tất cả / A / B / Không rõ; "Không rõ" chỉ có `gen.txt`; Tất cả đủ 3; pane hẹp thành dropdown, không cuộn ngang; menu rail "Xem thay đổi của lượt này" của Beta mở đúng bộ lọc B, vẫn 1 pane Changes; "Đánh dấu đã xem lượt này" xoá `last` của B, bộ lọc về Tất cả, `beta.ts` sang Không rõ; toast done có "Xem thay đổi" mở Workspace lọc B; Inbox Xong "Xem thay đổi" mở lọc A; terminal đóng thì bỏ tập lượt.
- Tiêu chí "sau 10 phút vẫn còn": unit `stores/turnEdits.test.ts` (fake timer qua `AFTER_TURN_MS`).

## Ảnh (`docs/qa/v104/WP-104h/`)
`v104h-01-filter-{light,dark}.png` (thanh Theo session, lọc Alpha), `v104h-02-toast-{light,dark}.png` (toast done), `v104h-03-narrow-light.png` (pane hẹp, dropdown).

## Lệch SPEC / quyết định nhỏ
- Key i18n `v104.turnChanges.*` (theo brief) thay vì `v104.turn.*` của SPEC 7.4; thêm `sessionTip`, `menuNone`, `command`, `doneToast`.
- Toast `done` trong app chỉ hiện khi lượt có file Edit / Write, theo switch `notifyDone` (trước đây `done` chỉ có `Notification` OS).
- Lọc session + "Đánh dấu đã xem" ở thanh review: chỉ đánh các file đang hiện, không đánh cả repo.
- Mục rail chỉ ở UI next (`LiveRail`); classic `WorkspaceSidebar` không truyền `turnChanges` nên không hiện.
- `openChanges` vẫn khớp pane theo `root` / `cwd` y như cũ: session có cwd là thư mục con của repo mà chưa có pane Changes cho thư mục đó có thể mở pane thứ 2 (hành vi có sẵn).
- E2E `v104-clash.cjs` lần chạy thứ 2 ghi lại ảnh `docs/qa/v104/WP-104d/*` (cùng kịch bản, nội dung tương đương).

## Rủi ro còn lại
- Restart app: tập lượt mất (chỉ `sessionStorage`), file rơi vào "Không rõ" (đúng SPEC).
- Agent chờ quyền giữa lượt (working → waiting) đóng lượt sớm; lượt tiếp theo gộp vào vì `last` chưa xem, nên không mất file.
- Số file trong toast đếm path đã Edit / Write của lượt, kể cả file sau đó đã revert (Changes không hiện file đó).

## QA tay Windows
Qua E2E Tauri dev ở trên. Chưa thử với 2 session Claude thật.

## Test macOS
- [ ] 2 session Claude cùng repo, mỗi bên sửa 1 file bằng Edit, 1 file đổi bằng lệnh shell: Changes hiện thanh "Theo session", lọc từng session đúng file, "Không rõ" có file shell.
- [ ] Chờ hơn 2 phút sau khi session xong: bộ lọc session vẫn còn tới khi bấm "Đánh dấu đã xem lượt này".
- [ ] Toast "{name} xong, đổi {n} file" (Settings: thông báo + Xong bật) bấm "Xem thay đổi" mở Changes đã lọc; đưa app xuống nền, `Notification` của webview Tauri bấm vào làm cùng việc (nếu webview có hiện).
- [ ] Rail chuột phải trên session: "Xem thay đổi của lượt này" mở đúng bộ lọc; session chưa sửa file thì mục bị tắt.
- [ ] Ctrl+K "Xem thay đổi lượt của session đang chọn" khi pane agent đang active.
- [ ] Path macOS phân biệt hoa thường (`/Users/...`): file khớp đúng session, rename (`git mv`) vẫn khớp.
- [ ] Pane Changes hẹp: dropdown, không cuộn ngang; sáng / tối.
