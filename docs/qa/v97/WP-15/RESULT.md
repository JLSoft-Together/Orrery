# WP-15 RESULT — 2026-11-11T22:05:39.421Z (30s)

| # | Mục | Kết quả | Kiểm |
|---|---|---|---|
| 1 | Bật remote, listener LAN riêng | ĐẠT | 5/5 |
| 2 | Ghép bằng QR + confirm native | ĐẠT | 3/3 |
| 3 | IK + hello + snapshot + ws.snapshot | ĐẠT | 5/5 |
| 4 | ws.open shell, desktop không đổi focus / view | ĐẠT | 6/6 |
| 5 | pty.attach + echo + claim | ĐẠT | 5/5 |
| 6 | ws.open khi desktop ở Monitor (H5) | ĐẠT | 3/3 |
| 7 | ws.close pane bận → confirm.ask tới điện thoại | LỖI | 7/8 |
| 8 | fs.list / fs.read / denied / UNC | ĐẠT | 5/5 |
| 9 | Thu hồi → 4401, nối lại bị từ chối | ĐẠT | 5/5 |
| 10 | Lặp 3–5 qua relay + thu hồi qua relay | ĐẠT | 9/9 |
| 11 | Tắt remote → listener đóng | ĐẠT | 3/3 |

## Log

```
PASS [1] remoteEnable(true) → enabled + listening — {"enabled":true,"listening":true,"port":4330,"relay":"connecting"} +5.0s
PASS [1] port = 4330 — 4330 +5.0s
PASS [1] addresses non-empty, all private IPv4 — ["192.168.3.27"] +5.0s
PASS [1] netstat: listener only on a private LAN IP (not 0.0.0.0 / 127.0.0.1 / ::) — 192.168.3.27:4330 +5.1s
PASS [1] netstat: every listener address is in status.addresses — 192.168.3.27:4330 +5.1s
PASS [2] remotePairStart → QR text — eyJ2IjoxLCJtYWNoaW5lIjoiM1dLU0l4ajJHd2JO +5.1s
PASS [2] phone_cli pair → pair.ok (native confirm answered ok by CCM_E2E_CONFIRM) — {"deviceId":"e2e-12329e5a5cdb","ev":"paired","machine":"3WKSIxj2GwbNMVm_0WV6KA","reply":{"machine":"3WKSIxj2GwbNMVm_0WV6KA","name":"DESKTOP-SMCL7FD","t":"pair.ok"}} +5.3s
PASS [2] status.devices has the new device — ["E2E Phone One"] +5.3s
NOTE e2e-confirms.log: Cho phép «E2E Phone One» điều khiển Orrery?
PASS [3] IK session established (phone_cli ready) +7.1s
PASS [3] hello → welcome ok (machine, version, minProto) — {"locale":"vi","machine":"3WKSIxj2GwbNMVm_0WV6KA","minProto":1,"name":"DESKTOP-SMCL7FD","ok":true,"proto":1,"re":"c1","relay":"https://orrery-relay.jinvu.worker +7.1s
PASS [3] event snapshot (protocolVersion + terminals) — seq 3 +7.2s
PASS [3] event ws.snapshot — rev 1794434714 tabs 1 +7.3s
PASS [3] ws.snapshot pane keys match the Pinia workspace store — phone ["pmwco3in80"] store ["pmwco3in80"] +7.3s
NOTE relay.token event: có (exp 1797026716)
PASS [4] ws.open → ok + paneKey — {"ok":true,"paneKey":"pmwco3kuu2","re":"c2"} +7.5s
PASS [4] new pane is in the store +7.5s
PASS [4] ui.view unchanged — workspace → workspace +7.5s
PASS [4] active pane / tab unchanged — pmwco3in80 → pmwco3in80 +7.5s
PASS [4] new pane runs a shell in the asked cwd — {"state":"running","cwd":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-QPujjA\\proj-a"} +7.5s
PASS [4] phone sees the new pane (ws.get) with a sessionId — 3132 ms {"agent":"shell","busy":false,"cwd":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-QPujjA\\proj-a","dirty":false,"key":"pmwco3kuu2","kind":"term","preset" +10.6s
PASS [5] pty.attach → ok — {"ok":true,"re":"c5","stream":1} +10.7s
PASS [5] event pty.ready {cols, rows} +10.7s
PASS [5] output contains the typed echo and its result (≥ 2 × orrery-e2e) — "-e2e\r\n\u001b[?25h\u001b[morrery-e2e\u001b]9;9;\"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-QPujjA\\proj-a\"\u001b\\\r\nPS C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-QPujjA\\proj-a> " +12.4s
PASS [5] desktop pane still running +12.4s
PASS [5] desktop view still unchanged +12.4s
PASS [6] ws.open while desktop is on Monitor → ok + paneKey — {"ok":true,"paneKey":"pmwco3pfv4","re":"c6"} +13.3s
PASS [6] pane reaches state running without opening Workspace — {"state":"running","cwd":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-QPujjA\\proj-b"} +13.6s
PASS [6] ui.view still monitor +13.6s
FAIL [7] a long command typed from the phone marks the pane busy (ranCommand) [finding F1 if FAIL] — ranCommand=false attach {"ok":true,"re":"c8","stream":2} +17.4s
PASS [7] a command typed on the desktop marks the pane busy (control for the confirm test) — ranCommand=true +18.2s
PASS [7] ws.close busy pane → ok + confirm cid (no immediate close) — {"confirm":"aa3bd915a40e29d37e5f2d66","ok":true,"re":"c9"} +18.3s
PASS [7] event confirm.ask reaches the phone (cid, title, ok, cancel) — {"cancel":"Huỷ","cid":"aa3bd915a40e29d37e5f2d66","detail":"Agent hoặc lệnh ở đó vẫn đang chạy và sẽ dừng.","ok":"Đóng","re":"c9","t":"confirm.ask","title":"Đóng proj-b?"} +18.4s
PASS [7] desktop shows no [role=alertdialog] +18.4s
PASS [7] pane still open while waiting +18.4s
PASS [7] confirm.answer ok → reply ok — {"ok":true,"re":"c10"} +18.5s
PASS [7] pane closed on the desktop +18.5s
PASS [8] fs.list in pane cwd → entry hello.txt — {"data":[{"directory":false,"name":"hello.txt","outOfRoot":false,"path":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-QPujjA\\proj-a\\hello.txt"}],"ok":true,"re":"c11"} +18.6s
PASS [8] fs.read inside root → text — {"data":{"bom":false,"eol":"lf","kind":"text","lang":"txt","mtimeMs":1794434709401.7517,"path":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-QPujjA\\proj-a\\hello.txt","size":17,"text":"orrery  +18.7s
PASS [8] fs.read outside every root → denied — {"error":"denied","ok":false,"re":"c13"} +18.7s
PASS [8] fs.read UNC → invalid — {"error":"invalid","ok":false,"re":"c14"} +18.8s
PASS [8] fs.read with .. out of the root → denied or invalid — {"error":"denied","ok":false,"re":"c15"} +18.9s
PASS [9] remoteRevoke(deviceId) → true +18.9s
PASS [9] phone gets device.revoked +19.0s
PASS [9] LAN session closed with 4401 — {"code":4401,"ev":"closed","stage":"session"} +19.0s
NOTE LAN reconnect: không vào được phiên ({"code":4401,"ev":"closed","stage":"msg2"})
PASS [9] reconnect with the revoked key refused with 4401 — {"code":4401,"ev":"closed","stage":"msg2"} +19.0s
PASS [9] device gone from status.devices +19.0s
NOTE relay status before item 10: online
PASS [10] status.relay = online — online +19.0s
PASS [10] second phone paired over LAN — {"deviceId":"e2e-a3cf265a931a","ev":"paired","machine":"3WKSIxj2GwbNMVm_0WV6KA","reply":{"machine":"3WKSIxj2GwbNMVm_0WV6KA","name":"DESKTOP-SMCL7FD","relay":"ht +19.2s
PASS [10] relay token available (pair.ok relayToken or relay.token event) — pair.ok exp 1797026728 relay https://orrery-relay.jinvu.workers.dev +21.5s
PASS [10] [3] relay: IK + hello → welcome — {"locale":"vi","machine":"3WKSIxj2GwbNMVm_0WV6KA","minProto":1,"name":"DESKTOP-SMCL7FD","ok":true,"proto":1,"re":"c1","r +22.4s
PASS [10] [3] relay: snapshot + ws.snapshot +22.6s
PASS [10] [4] relay: ws.open → paneKey — {"ok":true,"paneKey":"pmwco3wmz6","re":"c2"} +22.8s
PASS [10] [5] relay: pty.attach ok — {"ok":true,"re":"c5","stream":1} +26.3s
PASS [10] [5] relay: typed echo + result come back — "\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-QPujjA\\proj-b\"\u001b\\\r\nPS C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-QPujjA\\proj-b> " +28.3s
PASS [10] [9] relay: revoke closes the relay session with 4401 +28.8s
PASS [11] remoteEnable(false) → enabled false, listening false — {"enabled":false,"listening":false,"relay":"off"} +28.9s
PASS [11] netstat: nothing listens on 4330 +29.8s
PASS [11] status relay off — off +29.8s
NOTE keepAwake setting after off: true (OS power request không kiểm được: powercfg /requests cần admin)
```

## Lỗi tìm được (không sửa code sản phẩm)

- F1 (mục 7, nghi lỗi, trung bình): lệnh dài gõ từ điện thoại (`pty_in` qua hub) không làm pane shell "bận". `ranCommand` chỉ được đặt trong `TermPane.vue` `term.onData` (phím gõ trên desktop) nên `usePaneInfo.isBusy` = false, `ws.close` từ điện thoại đóng pane đang chạy `ping -n 60` mà không có `confirm.ask`. Tái hiện: ws.open shell, pty.attach, `pty_in` `ping -n 60 127.0.0.1\r`, đọc `ranCommand` (false), `ws.close`. Đường confirm đúng khi `ranCommand` true (đã kiểm: gõ trên desktop). File nghi: `apps/web/src/components/workspace/TermPane.vue:1036`, `apps/web/src/composables/usePaneInfo.ts:176`, `apps/web/src/composables/useRemotePhone.ts` (close).
- F2 (thấp): hàng thiết bị trong Settings → Điều khiển từ xa hiện chuỗi key thô `remoteV97.devices.platform.` khi `platform` ngoài `android|ios` (ở đây `cli`, từ hello / msg3 của phone_cli) — ảnh `settings-remote.png`. Thật ra điện thoại chỉ gửi android / ios nên ít gặp; nên có fallback. File: `apps/web/src/components/settings/remote/RemoteDeviceRow.vue:33`, `apps/web/src/i18n/remote-v97.ts:41,126`.
- F3 (quan sát): sau `ws.open`, `ws.get` đầu tiên trả summary cũ; pane mới (có `sessionId`) xuất hiện trong `ws.snapshot` sau khoảng 3 s (lần chạy khác > 8 s, 1 lần không thấy trong cửa sổ 8 s). Điện thoại thật phải đợi `ws.patch` / hỏi lại trước `pty.attach` (gateway đã có chờ summary 2 s cho attach). `ws.patch` chưa được kiểm trong E2E này. File nghi: `apps/web/src/composables/useRemoteSummary.ts` (nhịp publish).

## Ghi chú phạm vi kiểm

- Key Noise nằm trong OS keyring service `Orrery E2E v97` (còn lại trong Credential Manager); máy `machineId` đã đăng ký trên relay thật `https://orrery-relay.jinvu.workers.dev` (relay đang `online`, phone đi qua `wss://.../p/<machineId>?t=` bằng tunnel Node vì `phone_cli` không có TLS).
- Item 10: thu hồi qua relay ra 4401 (relay đã deploy bản close control).
- Item 11: `status.keepAwake` còn `true` sau khi tắt (đó là cài đặt người dùng, không phải trạng thái OS); yêu cầu power của hệ điều hành không kiểm được (`powercfg /requests` cần admin).
- Item 5: lần chạy đầu `echo orrery-e2e` được kiểm bằng số lần xuất hiện (≥ 2: dòng gõ và kết quả).
- Item 2: native confirm trả bằng `CCM_E2E_CONFIRM=ok` (hộp thật không hiện); log `e2e-confirms.log` có câu hỏi "Cho phép «E2E Phone One» điều khiển Orrery?".
- Ảnh Settings: `docs/qa/v97/WP-15/settings-remote.png` (đã xem: QR chưa hiện, listener 192.168.3.27:4330, Relay online, 1 thiết bị).
