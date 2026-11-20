# SPEC v107: giao thức chat điện thoại (WP C1)

Nguồn: `.planning/PLAN-v107-phone-chat.md` (quyết định 1 đến 8), mockup `.planning/site/public/chat-lai/`. Type TS: `packages/shared/src/remote.ts` (phần `REMOTE_CHAT_*`, `RemoteChat*`, `RemoteMenu*`, `RemotePane.menu`, lệnh `ws.menu`). Model Dart: `apps/mobile/lib/remote/chat_protocol.dart` (test `apps/mobile/test/chat_protocol_test.dart`, kiểm luôn hằng số khớp file TS).

Nguyên tắc: dùng lại lệnh / event có sẵn, chỉ thêm 3 lệnh `chat.*`, 1 lệnh `ws.menu`, 3 event `chat.*`, 1 field `menu` trên pane, 1 field `chat` trong reply `hello`. Field lạ bị bỏ qua ở cả 2 phía, mọi kích thước có max, không message nào vượt 256 KiB.

## 1. Phát hiện tính năng, tương thích

- Reply `hello` thêm `chat: 1` (`REMOTE_CHAT_PROTO`). Thiếu field = desktop cũ: điện thoại giữ hành vi cũ (bấm session mở terminal), không gửi `chat.*` / `ws.menu`.
- Điện thoại mới gửi `caps: [..., "chat"]` trong `hello`. Desktop chỉ gửi `chat.last` cho phiên có cap này. App cũ không gửi cap: không nhận event lạ nào.
- Desktop cũ nhận `chat.*` / `ws.menu` → `{ok:false, error:"denied"}` (nhánh `_` trong `apps/shell/src/remote/phone_ws.rs:159`). Điện thoại coi `denied` cho lệnh `chat.*` như desktop cũ, quay về terminal.
- `RemotePane.menu` là field tuỳ chọn: app cũ bỏ qua, desktop cũ không gửi.
- `control: false` (§4.1 docs): `chat.page` / `chat.watch` vẫn chạy (chỉ đọc); `ws.menu` / `ws.send` / `ws.keys` bị `denied` như các lệnh ghi khác.

## 2. Hộp thư (Inbox): đã có gì, thêm gì

Mỗi dòng = 1 pane agent (`kind: "term"`, `agent` = `claude` | `codex`; pane `shell` vẫn hiện nhưng không có câu cuối).

| Cột | Nguồn | Mới? |
|---|---|---|
| Tên | `ws.snapshot` `panes[].title` | có sẵn |
| Project | `tabs[].groups[].project` chứa pane, không có thì `tabs[].project`, không có thì `panes[].cwd`; lấy basename | có sẵn |
| Agent | `panes[].agent` (`claude` / `codex` / `shell`) | có sẵn |
| Trạng thái | `panes[].rail` (`asked` = chờ bạn, `working` / `subagents` = đang làm, `failed` = lỗi, còn lại = rảnh); `panes[].inbox.reason` để xếp Chờ bạn lên đầu | có sẵn |
| Thời gian | dashboard `snapshot.data.terminals[id = pane.terminalId].lastActivityAt` (đổi qua `terminal.updated`) | có sẵn |
| Context % | dashboard `agents[role = main, terminalId = pane.terminalId].contextPct` | có sẵn |
| Tool đang chạy | cùng agent main: `currentTool` + `currentToolSummary` (đã sanitize bởi `toolsum.rs`) + `currentToolStartedAt` | có sẵn |
| Câu cuối của agent | **event mới `chat.last`** | mới |

### Event `chat.last`

```json
{ "t": "chat.last", "full": true, "items": [
  { "paneKey": "p1", "text": "Lỗi do redirect_uri thiếu dấu / cuối nên Google từ chối callback.", "at": 1763600000000 },
  { "paneKey": "p3", "text": null }
] }
```

- `full: true`: gửi ngay sau `hello` (khi có cap `chat`) và sau `resync`; danh sách đủ, pane không có trong list = không có câu. Không có `full`: chỉ các pane đổi (upsert theo `paneKey`; `text: null` = xoá).
- `text`: câu trả lời xong gần nhất của agent (lượt kết thúc bằng `end_turn`), qua `clean()` (gộp khoảng trắng thành 1 dòng, bỏ ký tự điều khiển / bidi, che secret), cắt max `REMOTE_CHAT_LAST_MAX` = 160 ký tự có `…`. Lấy đoạn cuối (câu chốt thường ở cuối): bỏ code block, lấy đoạn văn cuối không rỗng.
- `at`: ms epoch lúc câu đó được ghi (timestamp record).
- Gom 1 s, max 1 event / s / phiên. Chỉ gửi khi text đổi.

## 3. Lịch sử chat theo session

### 3.1 Item

Mọi item có `id` (chuỗi opaque, max 64, duy nhất trong 1 `gen`), `seq` (số nguyên ≥ 0, thứ tự trong `gen`), `rev` (tăng khi item được cập nhật), `at?` (ms epoch). Kiểu theo `k`:

| `k` | Field | Ý nghĩa |
|---|---|---|
| `me` | `text`, `cut?` | câu chủ nhân gửi; max 4000 ký tự |
| `ai` | `text` (markdown), `cut?` | chữ agent trả lời (không thinking); max 8000 ký tự, dài hơn thì cắt + `cut: true` (điện thoại hiện "Xem đầy đủ trong terminal") |
| `tools` | `sum`, `items[]`, `more?` | 1 nhóm tool liên tiếp giữa 2 đoạn chữ, hiện gộp 1 dòng |
| `files` | `root`, `files[]`, `more?` | thẻ file đã sửa trong lượt |
| `sys` | `kind` (`stop` / `error` / `limit` / `compact`), `text?` | dòng hệ thống: bị dừng, lỗi API, chạm limit, compact; `text` max 300 |

`tools.sum` = số lần theo nhóm: `edit`, `run`, `read`, `search`, `web`, `agent`, `other` (`REMOTE_CHAT_TOOL_CATS`). Điện thoại tự dựng câu theo locale ("Đã sửa 3 file, chạy 2 lệnh"), desktop không gửi câu. Nhóm lạ phía điện thoại → `other`.
`tools.items[k] = {tool, cat, target?, err?}`: `tool` = tên tool max 40 (`Edit`, `Bash`, `mcp__orrery__open_session`, `apply_patch`…), `target` = tóm tắt ngắn max 120 (path tương đối, dòng đầu lệnh), `err: true` khi `tool_result.is_error`. Max 30 item, thừa thì `more` = số bị bỏ (giữ 30 cái đầu).
`files.files[k] = {path, add?, del?, out?}`: `path` tương đối `root` (dấu `/`), max 300; file ngoài `root` → path đã che (`~/…`) + `out: true` (không mở diff). Max 20, thừa thì `more`.

Ví dụ 1 lượt:

```json
[
  { "id": "u:7f3c", "k": "me", "seq": 81234, "rev": 0, "at": 1763600000000, "text": "Đăng nhập Google bị lỗi 500, xem giúp." },
  { "id": "t:toolu_01A", "k": "tools", "seq": 81990, "rev": 5, "at": 1763600004000,
    "sum": { "read": 4, "run": 1 },
    "items": [ { "tool": "Read", "cat": "read", "target": "auth/google.ts" }, { "tool": "Bash", "cat": "run", "target": "rg redirect_uri" } ] },
  { "id": "a:msg_01B", "k": "ai", "seq": 90412, "rev": 1, "at": 1763600020000, "text": "Lỗi do `redirect_uri` thiếu dấu `/` cuối..." },
  { "id": "f:7f3c", "k": "files", "seq": 81235, "rev": 2, "root": "C:\\work\\api",
    "files": [ { "path": "auth/google.ts", "add": 2, "del": 1 }, { "path": "test/auth.test.ts", "add": 14 } ] }
]
```


### 3.2 Lệnh

Lệnh do **gateway Rust** xử lý (không qua renderer), tính vào hạn mức viewer 120 / phút / thiết bị (§4.5 docs).

| Lệnh | Reply `data`-less (`{re, ok:true, ...}`) |
|---|---|
| `chat.page {paneKey, gen?, before?, limit?}` | `RemoteChatPage {paneKey, agent, gen, items[], older}` |
| `chat.watch {paneKey, limit?}` | như `chat.page` không `before` (trang mới nhất), rồi bắt đầu gửi `chat.items` cho pane đó |
| `chat.unwatch {paneKey}` | `{ok:true}` |

- `limit` mặc định 30 (`REMOTE_CHAT_PAGE`), max 50. Reply max 256 KiB (`REMOTE_CHAT_REPLY_BYTES`): đầy trước thì dừng sớm, `older` chỉ đúng chỗ dừng.
- Max 2 watch / phiên (`REMOTE_CHAT_WATCH_MAX`); thứ 3 → `limit`. Phiên đóng → nhả hết. `chat.watch` lại cùng pane = gửi lại trang mới nhất (dùng sau khi nối lại).
- Field trả lời theo đúng shape `{re, ok:true, paneKey, agent, gen, items, older}` (giống `ws.*` trả field phẳng).

Lỗi (mã có sẵn trong `RemoteErrorCode`):

| Mã | Khi |
|---|---|
| `notFound` | `paneKey` không có trong summary, hoặc pane không phải agent (shell, file, run…) |
| `invalid` | thiếu / sai field, `limit` ngoài 1..50, `before` không phải số nguyên ≥ 0 |
| `limit` | watch thứ 3 |
| `rate` | quá 120 lệnh viewer / phút |
| `offline` | core chưa lên (không có `tap`) |
| `denied` | desktop cũ |
| `internal` | đọc file lỗi |

Pane agent chưa có transcript (chưa gửi prompt nào): `ok:true`, `gen: ""`, `items: []`, `older: null`; khi transcript xuất hiện desktop gửi `chat.reset`.

### 3.3 Phân trang, `seq`, `gen`

- `gen` = định danh file nguồn (opaque, max 64), vd `<sessionId>:<n>`. Đổi khi: `/clear` hay resume sang file khác, file bị cắt ngắn (cỡ nhỏ hơn lần đọc trước), Codex sang thread khác.
- `seq` = byte offset của dòng JSONL đầu tiên tạo ra item (file chỉ append nên ổn định trong 1 `gen`). Item `files` lấy `seq` = offset dòng prompt mở lượt + 1 (ổn định dù lượt còn dài thêm; không dòng nào khác bắt đầu ở offset đó). Thứ tự = `seq` tăng dần, hoà thì `id`; riêng thẻ `files` điện thoại vẽ ở **cuối lượt** (ngay trước item `me` kế tiếp, hoặc cuối danh sách), như mockup. Dart: `ChatLog.view`.
- Trang = đọc ngược từ `before` (không có = cuối file) theo khúc, max 4 MiB quét / trang (`REMOTE_CHAT_SCAN_BYTES`), và **luôn mở trang tại dòng prompt của chủ nhân** để một lượt không bị cắt đôi giữa 2 trang. Trả item có `seq < before`. `older` = offset của dòng prompt đầu trang (gửi lại làm `before` để lấy trang cũ hơn); `null` = đã tới đầu file.
- Lượt dài hơn 4 MiB: trang mở giữa lượt, `id` vẫn lấy từ dòng đầu của chính item nên không trùng; điện thoại có thể thấy 1 lượt tách 2 khúc. Chấp nhận.
- `chat.page` có `gen` khác `gen` hiện tại: bỏ qua `before`, trả trang mới nhất với `gen` mới. Điện thoại thấy `gen` đổi thì xoá log cũ.

### 3.4 Cập nhật live, dedupe

- Event `chat.items {paneKey, gen, items[]}`: item mới hoặc item đã đổi (`ai` thêm khúc, `tools` thêm tool, `files` thêm file, `rev` tăng). Gom 300 ms (`REMOTE_CHAT_BATCH_MS`), max 64 KiB / event (`REMOTE_CHAT_EVENT_BYTES`), quá thì chia nhiều event; 1 item quá cỡ thì cắt `text` + `cut`.
- Điện thoại upsert theo `id`: thay khi `rev` mới ≥ `rev` đang có, bỏ khi nhỏ hơn. Reply `chat.watch` và `chat.items` có thể chồng nhau: upsert lo việc trùng. `chat.items` có `gen` khác `gen` đang giữ → bỏ, gọi lại `chat.watch` (Dart `ChatLog.stale`).
- Event `chat.reset {paneKey, gen}`: file nguồn đổi. Điện thoại xoá log, gọi lại `chat.watch`.
- Đang chạy tool: không có item riêng; dòng "Đang sửa Settings.vue 1:12" lấy từ dashboard `currentTool*` như hộp thư. Nhóm `tools` đang mở nhận `rev` mới mỗi lần thêm tool.
- Mất kết nối: khi nối lại, `chat.watch` lại pane đang mở; không có replay theo seq.
- Dữ liệu chỉ đi tới phiên đã watch, không broadcast.

## 4. Menu / xin quyền: bong bóng có nút

### 4.1 `RemotePane.menu` trong `ws.snapshot` / `ws.patch`

```json
"menu": {
  "sig": "1a2b3c4d", "kind": "permission", "agent": "claude",
  "question": "Do you want to proceed?",
  "items": [
    { "index": 1, "label": "Yes", "selected": true },
    { "index": 2, "label": "Yes, and don't ask again for npm test commands" },
    { "index": 3, "label": "No, and tell Claude what to do differently" }
  ],
  "numbered": true, "confident": true, "since": 1763600000000
}
```

- Nguồn: màn terminal sống, đúng đường v100 (docs §11as): `apps/web/src/lib/termMenu.ts:107` `parseTermMenu` → `TermPane.vue:467-470` báo vào `stores/menuQueue.ts` (cả pane ẩn, throttle 1 s `HIDDEN_MENU_THROTTLE_MS`) → `menuQueue.entryOf(paneKey)` = `{menu, question, since}`.
- `sig` = `menuSig(menu, question)` (`apps/web/src/lib/menuToast.ts:31`), cùng chữ ký toast dùng.
- `items[k]` từ `TermMenuItem`: `index` (1-based, duy nhất), `label`, `selected` (= highlight), `checked` (ask-multi), `other` (`isOther`), `submit` (`isSubmit`). Bỏ `row`. Max 12 item, label max 120, question max 300.
- Chỉ gửi khi `confident: true` hoặc `kind` là `permission` / `codex-approval`; không có menu → không có field (patch `remove`).
- Lệnh đang xin quyền (dòng `npm test -- auth` trong mockup): dashboard `agents[main].currentToolSummary` + `terminals[].waitingFor`; không thêm field.

### 4.2 Lệnh `ws.menu {paneKey, sig, index}`

- Do renderer xử lý như `ws.*` khác (gateway chuyển tiếp, có `rid`, tính 30 / phút).
- Renderer: `menuQueue.entryOf(paneKey)`; `menuSig` khác `sig` → `busy` (menu đã đổi, điện thoại vẽ lại theo snapshot mới); tìm item theo `index`, không có → `notFound`; `other` → `invalid` (bản 1 chưa hỗ trợ gõ chữ "Other" từ điện thoại; nút mở terminal); rồi `menuQueue.answer(paneKey, item)` (`stores/menuQueue.ts:63`, cùng đường pick của nút trên desktop và toast, `planMenuKeys` lo số / Up Down / Space / Enter). Reply `{ok:true}`.
- Hoàn tác 3 giây làm ở điện thoại: chỉ gửi `ws.menu` sau 3 s; bấm Hoàn tác trong 3 s thì không gửi gì.
- ask-multi: mỗi lần bấm 1 ô = 1 `ws.menu` (tick / bỏ tick), rồi `ws.menu` với item `submit`.
- `ws.keys` giữ nguyên cho app cũ và cho Dừng (`stop` = Esc khi đang làm) / `esc` (đóng menu). Không đổi.

## 5. Gửi tin, Dừng

- Gửi: `ws.send {paneKey, text}` có sẵn (max 2000). Luật chỉ submit khi rảnh bị bỏ ở WP C2; reply vẫn `{submitted}`.
- Dừng: `ws.keys {paneKey, keys: "stop", since}` có sẵn (`useSessionActions.ts:10` Esc, chỉ khi `working`).
- Không thêm lệnh mới.

## 6. Che secret, cái gì không bao giờ gửi

Mọi chuỗi trong `chat.*` và `menu` qua cùng bộ che của push (`apps/shell/src/remote/push_content.rs:206` `mask_text` → `core_desk::claude::mask::mask_hook_text`, `core-desk/src/claude/mask.rs:31`):

- Chuỗi 1 dòng (`chat.last.text`, `tools.items[].target`, `menu.question`, `menu.items[].label`, `sys.text`): `push_content::clean(text, &mask_text, max)` (`push_content.rs:129`): bỏ ký tự điều khiển + bidi (`pair::is_hidden`, `pair.rs:140`), gộp khoảng trắng, che, cắt có `…`.
- Chuỗi nhiều dòng (`me.text`, `ai.text`): giữ `\n` và `\t`, bỏ ký tự điều khiển khác + bidi, rồi `mask_text`, rồi `push_content::cut`. Không gộp khoảng trắng (markdown, code block).
- `target`: lấy từ `core_transcript::toolsum::tool_summary_of` (`toolsum.rs:174`; Codex `codex_tool_summary_of`, `toolsum.rs:231`), đã sanitize path, rồi `mask_text`.
- `menu`: renderer gửi nguyên văn trong summary; gateway che `question` + `label` trong `Board::publish` (`apps/shell/src/remote/workspace.rs:84`) trước khi hash / diff, để snapshot và patch luôn là bản đã che.
- Không bao giờ gửi: block `thinking` / `redacted_thinking`, nội dung `tool_result`, input thô của tool (chỉ `target`), record `isMeta`, `isSidechain` (subagent), `<system-reminder>…</system-reminder>` và các thẻ `<command-…>` trong prompt (prompt `/lệnh` hiện thành `/tên args`), env, ảnh / file đính kèm (chỉ chữ `[ảnh]`).
- Codex: bỏ message user do CLI chèn (bắt đầu bằng `<environment_context>`, `<user_instructions>`, AGENTS.md), bỏ `reasoning`.

## 7. Nguồn dữ liệu cho C3 (file:line)

Gateway (shell) chạy core trong cùng process (`apps/shell/src/serve.rs:88` `tap`).

**Claude**
- Đường dẫn transcript chính: `core_transcript::tracker::Tracker.transcript` (`apps/core/crates/core-transcript/src/tracker/mod.rs:100`, tìm ở `:213-216` qua `subs::find_transcript`, `tracker/subs.rs:28`). `MonTracker.core` đang private (`apps/core/crates/core-monitor/src/tracker.rs:30-37`): thêm accessor `transcript_path()`.
- Tìm tracker theo pane: `pane.terminalId` (summary) = key `Monitor.trackers`; cùng kiểu `send_tool_spans` (`core-monitor/src/monitor/actions.rs:225`) lọc theo `terminal_id`. Đường gọi từ gateway: theo mẫu `tool-spans.get` (`apps/shell/src/remote/dashboard.rs:344` `tap` → `client_message`, `core-serve/src/dispatch.rs:64` → `ClientMessage`, `core-http/src/client/mod.rs:47/268`): thêm `ClientMessage::ChatPage { terminal_id, before, limit }` trả `MonitorEvent` riêng, hoặc chỉ hỏi path rồi gateway tự đọc file (ưu tiên: đọc file trong `spawn_blocking` ở gateway, core chỉ trả path + session id).
- Parse dòng: tái dùng `record::parse_line` (`core-transcript/src/record.rs:143`) và logic `extract.rs`: `assistant` (`:100`, text block qua `texts_of` `:75`, tool_use `:132-146`), `user` (`:163`, `isMeta`, `tool_result.is_error` `:175`), `extract_signals` (`:188`). Không đổi `Signal` (tracker đang dùng); viết hàm mới `chat_items_of(line, offset)` cạnh đó.
- Live: tail riêng bằng `JsonlTailer` (`core-transcript/src/tail.rs:31`) mở ở offset cuối lúc `chat.watch`, đọc khi dashboard tap có event cho `terminalId` đó (`agent.updated`, `terminal.updated`, `tool.span`) và mỗi 5 s dự phòng khi đang watch; không poll khi không ai watch.
- Câu cuối (`chat.last`): `Monitor::reply_of(terminal_id)` (`core-monitor/src/monitor/actions.rs:75`, từ `state.rs:154` `last_reply`, max `REPLY_MAX` 3000, `extract.rs:10`); đúng chỗ `core-serve/src/alerts.rs:56` đang đọc. Thời điểm: `TerminalSession.lastActivityAt` lúc reply đổi.

**Codex**
- Rollout: `core-monitor/src/codex/threads.rs:143` `add_thread` (path file), tailer `:168`; terminal standalone `codex:<threadId>` (`codex/publish.rs:102`).
- Extractor hiện bỏ qua message (`core-transcript/src/codex/extract.rs:201` chỉ nhận `response_item` có `call_id`; docs `codex-integration.md:35` liệt kê `message`, `reasoning` là bỏ qua). C3 phải **soi rollout của Codex 0.162 đã cài trước khi code** (luật CLAUDE.md "inspect the installed version first"): ứng viên `event_msg` `user_message` / `agent_message` (chữ sạch, không có context chèn) hoặc `response_item` `message` role `user` / `assistant` content `input_text` / `output_text`. Ghi kết quả vào `docs/codex-integration.md`.
- Tool: `function_call` / `custom_tool_call` / `local_shell_call` (`extract.rs:201-209`), `target` từ `codex_tool_summary_of`. `apply_patch`: đếm `+` / `-` theo từng `*** Update File:` / `*** Add File:` cho thẻ `files`.
- Câu cuối: chưa có `last_reply` cho Codex; C3 thêm từ message assistant cuối của lượt (`task_complete`).

**Phân loại tool → `cat`**: `Edit` / `Write` / `MultiEdit` / `NotebookEdit` / `apply_patch` → `edit`; `Bash` / `PowerShell` / `shell` / `exec_command` / `local_shell` → `run`; `Read` / `NotebookRead` → `read`; `Grep` / `Glob` / `LS` / `ToolSearch` → `search`; `WebFetch` / `WebSearch` → `web`; `Task` / `Agent` → `agent`; còn lại (MCP…) → `other`.

**Thẻ `files`**: gom theo lượt (từ prompt tới prompt kế), 1 dòng / path, cộng dồn. `Edit`: `add` = số dòng `new_string`, `del` = số dòng `old_string` (MultiEdit cộng từng edit); `Write`: `add` = số dòng `content`, không có `del` (không biết file cũ); `NotebookEdit`: chỉ path. Đây là số ước lượng từ input tool, không phải git; không gửi nội dung.
**Diff khi bấm**: dùng lệnh viewer có sẵn `git.diff {root, path}` (`apps/shell/src/remote/viewer_map.rs:74`, kênh `ccm:changes-diff`) với `root` = `files.root` (cwd pane, đã nằm trong root được phép). Diff là working tree hiện tại so với HEAD, không phải riêng lượt đó; file đã commit / không có thay đổi → điện thoại hiện "Không còn thay đổi".

**Menu (renderer, C3 phần web)**: `apps/web/src/lib/remoteSummary.ts:41` `paneOf` thêm `menu` từ `menuQueue.entryOf`; `useRemoteSummary.ts:48` truyền vào; `useRemotePhone.ts:212` thêm `handleOrrery('ws.menu', …)` (theo `itemForAnswer`, `menuToast.ts:76`); `packages/shared/src/orreryApi.ts:289` thêm `'ws.menu': fromPhone('ws.menu')`; gateway `apps/shell/src/remote/workspace.rs:14` `WS_COMMANDS` và `phone_ws.rs:148` thêm `"ws.menu"`. Parser `parseRemoteCommand` đã có case `ws.menu` (WP C1).
**Hello**: `apps/shell/src/remote/ctx.rs:324` thêm `"chat": 1`.

## 8. Giới hạn (tóm tắt)

| Hằng (`remote.ts`) | Giá trị |
|---|---|
| `REMOTE_CHAT_PAGE` / `_PAGE_MAX` | 30 / 50 item |
| `REMOTE_CHAT_REPLY_BYTES` | 256 KiB / reply |
| `REMOTE_CHAT_EVENT_BYTES` | 64 KiB / event |
| `REMOTE_CHAT_SCAN_BYTES` | 4 MiB quét / trang |
| `REMOTE_CHAT_BATCH_MS` | 300 ms |
| `REMOTE_CHAT_WATCH_MAX` | 2 / phiên |
| `REMOTE_CHAT_ME_MAX` / `_AI_MAX` / `_SYS_MAX` | 4000 / 8000 / 300 ký tự |
| `REMOTE_CHAT_TOOLS_MAX` / `_FILES_MAX` | 30 / 20 |
| `REMOTE_CHAT_TARGET_MAX` / `_TOOL_NAME_MAX` / `_PATH_MAX` | 120 / 40 / 300 |
| `REMOTE_CHAT_LAST_MAX` | 160 |
| `REMOTE_CHAT_ID_MAX` | 64 |
| `REMOTE_MENU_ITEMS_MAX` / `_LABEL_MAX` / `_QUESTION_MAX` / `_SIG_MAX` | 12 / 120 / 300 / 16 |

Relay: mọi message ≤ 256 KiB, khung Noise 60 KiB tự chia (§3 docs), không cần đổi relay.

## 9. Test C3 nên có

- Rust: transcript mẫu → item đúng thứ tự, không thinking / tool_result / isMeta / sidechain / system-reminder; secret trong prompt + reply + target bị che; trang mở tại prompt; `before` / `older` đi hết file không trùng không sót; file bị cắt → `gen` đổi + `chat.reset`; watch thứ 3 → `limit`; desktop không cap `chat` → không có `chat.last`.
- Web: `remoteSummary` có `menu` khi `menuQueue` có entry; `ws.menu` sai `sig` → `busy`, `other` → `invalid`, đúng → gọi picker 1 lần; `rid` trùng không gọi 2 lần.
- Docs: cập nhật `docs/remote-integration.md` §4.1 (`chat`, cap), §4.3 (`menu`, `ws.menu`), mục mới §4.7 Chat.
