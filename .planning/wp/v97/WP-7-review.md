# WP-7 review

packages/shared/src/remote.ts:137-160 — vừa — `parseRemoteCommand` không giới hạn charset của `paneKey` / `tabKey` / `device` (chỉ độ dài ≤ 200/100), `rename.name` cho phép newline / control char; `device` đi vào log + inbox note — fix: whitelist `[\w.:-]` cho key, bỏ control char ở `name` và `device`.
packages/shared/src/remote.ts:176-178 — vừa — `confirmed` đọc từ `raw.cmd.confirmed` (lồng trong cmd). Nếu gateway forward nguyên `cmd` của phone thì phone tự set `confirmed:true` để bỏ qua `needsConfirm` của `ws.close` — fix: renderer chỉ tin `confirmed` ở cấp `call`, gateway (WP-5) phải strip `confirmed` từ message phone và tự set; ghi vào docs §4.3.
apps/web/src/lib/remoteSummary.ts:37,64 — thấp — summary gửi `cwd` đầy đủ của pane + `tab.project` (path tuyệt đối) sang phone; brief ghi "không đưa path" nhưng docs/remote-integration.md:66-67 liệt kê `cwd`, `project` — fix: chốt với owner; nếu chỉ cần hiển thị thì gửi baseName, còn `ws.open` cần path thì giữ nhưng ghi rõ trong docs.
apps/web/src/composables/useRemotePhone.ts:65-70 — thấp — `keepFocus` gọi `workspace.activate(before)` sau open/move/reopen: activate có thể reset attention / `lastSeen` của pane đang focus (desktop mất dấu "chưa đọc") — fix: restore `activeKey` trực tiếp không đi qua `activate`, hoặc test rằng attention không đổi.
apps/web/src/composables/useRemotePhone.ts:100-106 — thấp — nếu `pendingClose` của desktop đang chứa key này thì lệnh phone huỷ / sửa sheet desktop (`cancelClose` / gán lại) — không mở sheet mới nên đúng yêu cầu, nhưng đổi state sheet đang hiện của desktop; chấp nhận được, nên có test.
apps/web/src/composables/useRemoteSummary.ts:12-75 — thấp — watcher `hashed` / `connected` và `setTimeout` không có đường stop (singleton `installed`); `build(0)` chạy lại mỗi thay đổi reactive khi có phone — fix: trả về hàm dispose, hoặc chấp nhận vì App sống suốt đời window.
apps/web/src/lib/ptyClient.ts:150-160 / TermPane.vue:780-794 — thấp — `claim` gửi cho mọi keydown không phải modifier (gồm Ctrl+C copy, phím điều hướng) khi `resizedByOther`; không gửi trên `onData` query reply (đúng) — fix tuỳ chọn: bỏ qua Ctrl/Meta+C khi có selection.
apps/web/src/i18n/en.ts, vi.ts — thấp — WP ghi "cấm đụng en/vi" (hot file), bản này đã sửa (chỉ thêm import + spread) — fix: để orchestrator xác nhận khi merge, tránh xung đột với WP khác.

## Đã kiểm, ổn
- Không có comment mới trong diff / file mới (rg).
- `ws.close` không đi `pendingClose` để mở sheet: `requestClose(..., () => false)` luôn đóng ngay; busy / dirty chưa confirmed trả `needsConfirm`; dirty phải `saveFiles` ok mới đóng.
- `ws.open` / `move` / `reopen` không gọi `ui.setView`; mount hidden trong App.vue chỉ set `workspaceOpened = true` (khai báo trước dòng 93, không TDZ), `ui.view` không đổi; `session.open` vẫn chỉ nhận `mcp` / `url`.
- Summary: không có url / prefill / env / path file / prompt; `cwd` / `project` xem mục thấp trên. Không publish khi `connected === 0` (computed trả '', timer bị clear, `publish` kiểm lại); hash không gồm `rev`, rev tăng chỉ khi hash đổi; reset `lastHash` khi 0 -> có phone.
- `resized` chỉ set cờ `sized` + `resizedByOther`; desktop 1 socket không nhận `resized` nên hành vi TermPane không đổi; `onBeforeUnmount` dọn cờ.
- `claim` gọi ở `onKey` (keydown, bỏ modifier / parked) và `paste()`; không ở `onData`, không ở `term.paste` do prefill / compact (đi `onData`).
- Parser: text > 2000 bị từ chối (ws.send, ws.open.prompt), `resume` qua `isAgentSessionId`, `since` finite, keys whitelist, cwd cấm newline / NUL, preset whitelist (không `deepseek`).
- `OPEN_POLICY.phone` = `submit:false, askSubmit:true, limit:true, dock:false`; submit chỉ khi `cmd.submit === true`; các source cũ (menu, palette, mcp, url, telegram, restore) và spec cũ không đổi.
- i18n: `remote-v97.ts` có en + vi, có test parity; `settings.pages.remote` có cả en / vi.
- Vitest 6 file liên quan: 37 pass.
