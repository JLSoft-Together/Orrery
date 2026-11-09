# WP-v87-2-A2 — báo cáo

**Trạng thái: DONE.** E2E `v49-open.cjs`, `v80-mcp.cjs`, `shell-agent.cjs` **CHƯA KIỂM**: phải build stage, mà luật không cho build để verify. Giao diện **CHƯA NHÌN**.

## Đã đổi

| File | Việc |
|---|---|
| `apps/desktop/src/pty/broker.ts` | `toolHost(token)`: chỉ pane đã `spawned`, chưa `exited` (map `tokens`, xoá cùng pane / chỗ giữ quá hạn / dispose). `take` giới hạn tần suất theo pane, `notify` qua `NoteQueue` (dispose khi `removed`), `openSession` dùng thẳng `openFrom`, `checkoutRoots`, `page` (max 8 s), `desk` (`deskReplyMs` 40 s / 200 s / 600 s, có `pid` + `name`). Thêm option `page` / `desk`. `ORRERY_API` lấy `port()` = port MCP của main |
| `apps/desktop/src/pty/mcp.ts` | `mcpHttpHandler` + `BODY_MAX`: chuyển nguyên `onRequest` / `readBody` cũ của host sang. Gate → tra token → 401 trước khi đọc body → 413 / 400 → `handleMcp` |
| `apps/desktop/src/pty/supervisor.ts` | MCP HTTP server `127.0.0.1:0` trong main: mở 1 lần trước lần fork đầu, đóng ở `stop()`. Port riêng, khác port `/pty`. Lỗi listen thì ghi log và vẫn mở terminal được. `ask` (tool → `MCP_TOOL_ACTION` → `onOrrery`, cùng lỗi `failed` / `unavailable` như `relayMcp` cũ). Bỏ `relayOpenFrom` / `relayRoots` / `relayMcp` |
| `apps/desktop/src/pty/host.ts` (594 → 464 dòng) | Bỏ toàn bộ MCP: HTTP `/mcp`, tokens, rates, NoteQueue, relay, `DEEPSEEK`, `APP_VERSION`. Request HTTP không phải `/pty` trả 404 |
| `packages/shared/src/ptyControl.ts` | Bỏ `PtyNodeHostRelay` / `PtyNodeHostRelayReply` |
| `apps/desktop/src/main.ts` | **Không đổi**: `PtySupervisor` giữ nguyên options, `onOrrery` vẫn là dispatcher |
| `.code_index/desktop.md`, `protocol.md`, `_meta.md` | Cập nhật cho A1 + A2: host mỏng, `PtyBroker`, contract `ptyControl.ts`, MCP ở main, kênh page / desk mới, test mới |
| Test | `mcp-http.test.ts` mới (2). `pty-broker.test.ts` thêm 4 (token → tool host, rate, page / desk mang caller + pid, không trả lời → `unavailable`). `pty-supervisor.test.ts`: MCP gọi vào port main, port `/pty` trả 404 cho `/mcp`, `notify_owner` tới `onOrrery` với `{source:'mcp', pane}`, token của pane đã kill → 401, sau `stop` không còn MCP |

## Tiêu chí A2: 4/4 (+ E2E chưa kiểm)

| # | Tiêu chí | Kết quả |
|---|---|---|
| 1 | Pane Codex / Claude gọi được `open_session`, `notify_owner`, `list_sessions` | ✓ theo test (open_session end to end, notify_owner tới `onOrrery`; `list_sessions` đi cùng đường `page`). Pane thật: CHƯA KIỂM |
| 2 | Token sai → 401 trước khi đọc body | ✓ (`mcp-http.test.ts` gửi body 256 KB với token sai → 401, không phải 413) |
| 3 | `ORRERY_API` port ≠ port `/pty` | ✓ (`pty-supervisor.test.ts`) |
| 4 | Bỏ relay `mcp-open` / `mcp-roots` / `mcp` / `mcp-desk` / `mcp-reply` khỏi host + contract | ✓ (grep host.ts không còn `mcp` / `token` MCP) |

## Audit / Review / Test / Nhìn

- **Audit:** chỉ sửa file trong scope. Không đụng `apps/core/**`, `build.mjs`, `electron-builder.yml`, `.github/**`. Không thêm comment, không chạy git.
- **Review:**
  - **Tên tool, schema, instructions không đổi:** vẫn `handleMcp` + `mcpToolsFor(CCM_DEEPSEEK_FEATURE)`.
  - **Timeout:** page 8 s và desk 40 / 200 / 600 s giữ nguyên (trước ở host, nay ở broker).
  - **Lỗi:** action lạ → `failed`, thiếu `onOrrery` → `unavailable`, throw → `failed`.
  - **Pane exit:** token hết hiệu lực (401) như cũ.
  - **Host chết:** `broker.dispose()` thì token cũ trả 401. MCP server vẫn sống; host mới có broker mới.
  - **Event loop của main:** body ≤ 64 KB, batch ≤ 16 giữ nguyên; `wait_session` / `free_port` chờ bằng promise, không chặn.
  - **Khi `stop`:** `closeAllConnections` cắt luôn các call MCP còn đang chờ, lúc thoát app thì chấp nhận được.
- **Test (lệnh → kết quả):**
  - `npm run typecheck -w @ccm/desktop` → pass.
  - `npm run typecheck -w @ccm/shared` → pass.
  - `vitest run test/pty-broker.test.ts test/mcp-http.test.ts test/pty-supervisor.test.ts test/pty-host-stdio.test.ts test/pty-control.test.ts` → 51/51.
  - `npm test -w @ccm/desktop -- --maxWorkers=6` → 125 file / 1484 test pass. Đã gồm `mcp*.test.ts` và `deepseek-mcp.test.ts`.
- **Nhìn:** CHƯA NHÌN.

## Contract cuối

Giống A1 vòng 2, chỉ bỏ phần relay riêng của host Node. Host Node và Rust giờ nói đúng một contract `PtyMainToCore` / `PtyCoreToMain`. Trường `init.appVersion` vẫn giữ, dù host Node không còn dùng.

## Việc cho WP sau

- **C (Rust):** không cần làm MCP. Không cần đọc `ORRERY_TOKEN` trong env pane.
- **E2:** chạy E2E `v49-open.cjs`, `v80-mcp.cjs`, `shell-agent.cjs`, `p2`–`p6` trên dev build với `--user-data-dir` tạm. Đo xem MCP chạy trong main có làm event loop main chậm thấy được không: `wait_session` dồn nhiều call cùng lúc.
- `docs/claude-code-integration.md` §11w (MCP ở host) nên ghi lại theo nơi chạy mới. File này ngoài scope, con chưa sửa.

## Commit message đề xuất

```
♻️ MCP orrery server moves to Electron main: PtyBroker maps pane tokens to tool hosts, page/desk calls go straight to onOrrery, PTY host drops MCP and relays
```

## Vòng 2 (7 điểm reviewer) — DONE

| # | Sửa | Chỗ |
|---|---|---|
| 1 | MCP server không listen được thì ghi log `mcp server failed, orrery MCP off for this host`, đặt `mcpPort` = 0, `mcpStarting` = null, đóng server, để lần start host sau thử lại. Broker gặp `port()` = 0: pane không có `ORRERY_API` / `ORRERY_TOKEN` / `ORRERY_MCP_CONFIG`, không gắn `--mcp-config` / `-c mcp_servers.orrery`, không ghi file deepseek MCP. Handler `error` lúc listen gỡ ra sau khi listen thành công | `supervisor.ts` `startMcp`, `broker.ts` `resolve` |
| 2 | Đường tool MCP hết fs sync: `resolveInside`, `remoteLinkBelow`, `mtimeOf`, `realOf` chuyển sang async (`lstat` / `readlink` / `stat` từ fs/promises, realpath = `promisify(fs.realpath)` bản JS, giữ nguyên cách xử lý ổ map / subst); `handBack` dùng `stat` async; `whoIsEditing.withMtime` async. Cập nhật 5 chỗ gọi | `mcpTools/paths.ts`, `handBack.ts`, `whoIsEditing.ts`, `openInWorkspace.ts`, `previewLayout.ts`, `telegramSend.ts`, test `mcp-tools.test.ts` |
| 3 | `page` / `desk` kiểm `alive(s)` (`!disposed && panes.get(id) === s`) trước khi gọi và khi nhận kết quả; sai thì trả `unavailable` | `broker.ts` `askMain` |
| 4 | Pane đã xoá: `notify` trả `{ok:false, error:'unavailable'}`, không tạo `NoteQueue` | `broker.ts` `toolHost` |
| 5 | Callback listen chạy khi đã `stopped` thì `server.close()` ngay | `supervisor.ts` |
| 6 | `within` thành method: timer `unref`, tập `waits`; `dispose()` cho mọi call đang chờ trả `unavailable` ngay và clear timer | `broker.ts` |
| 7 | Body vượt `BODY_MAX`: gỡ listener `data` + `pause`, trả 413 `Connection: close` rồi `req.destroy()` | `mcp.ts` |
| docs | §11w: MCP chạy trong Electron main từ v87 bước 2; `ORRERY_API` = port MCP của main; không listen được thì pane không có `ORRERY_*`; host trả 404 cho `/mcp`; `wait_session` chạy ở main; file deepseek MCP theo port server MCP | `docs/claude-code-integration.md` |

**Test thêm:**
- (1) Port 0 thì claude / codex không có `ORRERY_*`, không có MCP flag.
- (3) + (4) Pane bị xoá trước hoặc trong lúc chờ: page trả `unavailable`, page handler chỉ được gọi 1 lần, notify trả `unavailable`.
- (6) `dispose` cho desk call đang treo trả `unavailable` ngay, `waits` về rỗng.
- (7) Body chunked tới 4 MB với token đúng: kết nối bị đóng trước khi gửi hết.

**Lệnh → kết quả:**
- `npm run typecheck -w @ccm/desktop` / `-w @ccm/shared` → pass.
- `vitest run test/mcp-tools.test.ts test/mcp-hand-back.test.ts test/mcp-desk.test.ts test/telegram-review.test.ts test/telegram-buttons.test.ts` → 77/77.
- `vitest run test/pty-broker.test.ts test/mcp-http.test.ts test/pty-supervisor.test.ts` → 40/40.
- `npm test -w @ccm/desktop -- --maxWorkers=6` → 125 file / 1488 test pass.

Không build, không đụng `apps/desktop/stage`. E2E vẫn CHƯA KIỂM (main đang chạy).

**Giới hạn còn lại:**
- Test (1) chỉ kiểm phía broker. Nhánh listen lỗi trong supervisor chưa có test tự động vì cần giả lỗi `listen`.
- Test (7) nhận status 0 hoặc 413, vì client có thể bị cắt kết nối trước khi đọc được response.
