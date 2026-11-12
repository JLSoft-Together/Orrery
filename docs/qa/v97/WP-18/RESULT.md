# WP-18 RESULT — 2026-11-12T01:09:14.926Z (25s)

| # | Mục | Kết quả | Kiểm |
|---|---|---|---|
| 1 | Bật remote, listener LAN riêng | ĐẠT | 5/5 |
| 2 | Ghép bằng QR + confirm native | ĐẠT | 3/3 |
| 3 | IK + hello + snapshot + ws.snapshot | ĐẠT | 5/5 |
| 4 | ws.open shell, desktop không đổi focus / view | ĐẠT | 6/6 |
| 5 | pty.attach + echo + claim | ĐẠT | 5/5 |
| 6 | ws.open khi desktop ở Monitor (H5) | ĐẠT | 3/3 |
| 7 | ws.close pane bận → confirm.ask tới điện thoại | ĐẠT | 8/8 |
| 8 | fs.list / fs.read / denied / UNC | ĐẠT | 5/5 |
| 9 | Thu hồi → 4401, nối lại bị từ chối | ĐẠT | 5/5 |
| 10 | Lặp 3–5 qua relay + thu hồi qua relay | ĐẠT | 9/9 |
| 11 | Tắt remote → listener đóng | ĐẠT | 3/3 |
| 12 | F3 ws.patch ≤ 1 s + log lệnh + tên nền tảng | ĐẠT | 5/5 |

## Log

```
PASS [1] remoteEnable(true) → enabled + listening — {"enabled":true,"listening":true,"port":4330,"relay":"connecting"} +5.2s
PASS [1] port = 4330 — 4330 +5.2s
PASS [1] addresses non-empty, all private IPv4 — ["192.168.3.27"] +5.2s
PASS [1] netstat: listener only on a private LAN IP (not 0.0.0.0 / 127.0.0.1 / ::) — 192.168.3.27:4330 +5.4s
PASS [1] netstat: every listener address is in status.addresses — 192.168.3.27:4330 +5.4s
PASS [2] remotePairStart → QR text — eyJ2IjoxLCJtYWNoaW5lIjoiYlY4cjlOUlBVaFJZ +5.4s
PASS [2] phone_cli pair → pair.ok (native confirm answered ok by CCM_E2E_CONFIRM) — {"deviceId":"e2e-3af42e33b0d2","ev":"paired","machine":"bV8r9NRPUhRYaRpcf2yvFA","reply":{"machine":"bV8r9NRPUhRYaRpcf2yvFA","name":"DESKTOP-SMCL7FD","t":"pair.ok"}} +5.7s
PASS [2] status.devices has the new device — ["E2E Phone One"] +5.7s
NOTE e2e-confirms.log: Cho phép «E2E Phone One» điều khiển Orrery?
PASS [3] IK session established (phone_cli ready) +7.5s
PASS [3] hello → welcome ok (machine, version, minProto) — {"locale":"vi","machine":"bV8r9NRPUhRYaRpcf2yvFA","minProto":1,"name":"DESKTOP-SMCL7FD","ok":true,"proto":1,"re":"c1","relay":"https://orrery-relay.jinvu.worker +7.5s
PASS [3] event snapshot (protocolVersion + terminals) — seq 3 +7.5s
PASS [3] event ws.snapshot — rev 1794445735 tabs 1 +7.6s
PASS [3] ws.snapshot pane keys match the Pinia workspace store — phone ["pmwcunq560"] store ["pmwcunq560"] +7.6s
NOTE relay.token event: có (exp 1797037737)
PASS [4] ws.open → ok + paneKey — {"ok":true,"paneKey":"pmwcunse82","re":"c2"} +7.7s
PASS [4] new pane is in the store +7.7s
PASS [4] ui.view unchanged — workspace → workspace +7.7s
PASS [4] active pane / tab unchanged — pmwcunq560 → pmwcunq560 +7.7s
PASS [4] new pane runs a shell in the asked cwd — {"state":"running","cwd":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-nnTh8W\\proj-a"} +8.0s
NOTE F3: ws.open gửi → pane mới có trong summary điện thoại sau 324 ms (qua ws.patch)
PASS [4] phone sees the new pane (ws.get) with a sessionId — 0 ms {"agent":"shell","busy":false,"cwd":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-nnTh8W\\proj-a","dirty":false,"key":"pmwcunse82","kind":"term","preset" +8.0s
PASS [5] pty.attach → ok — {"ok":true,"re":"c3","stream":1} +8.0s
PASS [5] event pty.ready {cols, rows} +8.0s
PASS [5] output contains the typed echo and its result (≥ 2 × orrery-e2e) — "-e2e\r\n\u001b[?25h\u001b[morrery-e2e\u001b]9;9;\"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-nnTh8W\\proj-a\"\u001b\\\r\nPS C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-nnTh8W\\proj-a> " +9.8s
PASS [5] desktop pane still running +9.8s
PASS [5] desktop view still unchanged +9.8s
PASS [6] ws.open while desktop is on Monitor → ok + paneKey — {"ok":true,"paneKey":"pmwcunuog4","re":"c4"} +10.7s
PASS [6] pane reaches state running without opening Workspace — {"state":"running","cwd":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-nnTh8W\\proj-b"} +10.7s
PASS [6] ui.view still monitor +10.7s
PASS [7] a long command typed from the phone marks the pane busy (ranCommand) [finding F1 if FAIL] — ranCommand=true attach {"ok":true,"re":"c5","stream":2} +14.5s
PASS [7] a command typed on the desktop marks the pane busy (control for the confirm test) — ranCommand=true +15.3s
PASS [7] ws.close busy pane → ok + confirm cid (no immediate close) — {"confirm":"0a19ebdf704f526938d0d0d6","ok":true,"re":"c6"} +15.4s
PASS [7] event confirm.ask reaches the phone (cid, title, ok, cancel) — {"cancel":"Huỷ","cid":"0a19ebdf704f526938d0d0d6","detail":"Agent hoặc lệnh ở đó vẫn đang chạy và sẽ dừng.","ok":"Đóng","re":"c6","t":"confirm.ask","title":"Đóng proj-b?"} +15.4s
PASS [7] desktop shows no [role=alertdialog] +15.4s
PASS [7] pane still open while waiting +15.4s
PASS [7] confirm.answer ok → reply ok — {"ok":true,"re":"c7"} +15.5s
PASS [7] pane closed on the desktop +15.5s
PASS [12] F3: pane mới tới điện thoại ≤ 1 s sau ws.open, qua ws.patch (không cần ws.get) — 324 ms ws.patch +15.5s
NOTE log lệnh: ["đóng proj-b","mở proj-b","mở proj-a"]
PASS [12] log có dòng mở pane (ws.open) — ["đóng proj-b","mở proj-b","mở proj-a"] +15.5s
PASS [12] log có dòng đóng pane (ws.close) — ["đóng proj-b","mở proj-b","mở proj-a"] +15.5s
PASS [12] Settings hiện log lệnh (data-remote-log có ≥ 2 dòng) +16.7s
PASS [12] F2: hàng thiết bị hiện "Không rõ" thay vì key thô — E2E Phone One
Worktree +16.7s
PASS [8] fs.list in pane cwd → entry hello.txt — {"data":[{"directory":false,"name":"hello.txt","outOfRoot":false,"path":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-nnTh8W\\proj-a\\hello.txt"}],"ok":true,"re":"c8"} +17.1s
PASS [8] fs.read inside root → text — {"data":{"bom":false,"eol":"lf","kind":"text","lang":"txt","mtimeMs":1794445729672.1892,"path":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-nnTh8W\\proj-a\\hello.txt","size":17,"text":"orrery  +17.2s
PASS [8] fs.read outside every root → denied — {"error":"denied","ok":false,"re":"c10"} +17.3s
PASS [8] fs.read UNC → invalid — {"error":"invalid","ok":false,"re":"c11"} +17.3s
PASS [8] fs.read with .. out of the root → denied or invalid — {"error":"denied","ok":false,"re":"c12"} +17.4s
PASS [9] remoteRevoke(deviceId) → true +17.4s
PASS [9] phone gets device.revoked +17.4s
PASS [9] LAN session closed with 4401 — {"code":4401,"ev":"closed","stage":"session","at":1794445747094} +17.5s
NOTE LAN reconnect: không vào được phiên ({"code":4401,"ev":"closed","stage":"msg2","at":1794445747215})
PASS [9] reconnect with the revoked key refused with 4401 — {"code":4401,"ev":"closed","stage":"msg2","at":1794445747215} +17.6s
PASS [9] device gone from status.devices +17.6s
NOTE relay status before item 10: online
PASS [10] status.relay = online — online +17.6s
PASS [10] second phone paired over LAN — {"deviceId":"e2e-2091b728f683","ev":"paired","machine":"bV8r9NRPUhRYaRpcf2yvFA","reply":{"machine":"bV8r9NRPUhRYaRpcf2yvFA","name":"DESKTOP-SMCL7FD","relay":"ht +17.7s
PASS [10] relay token available (pair.ok relayToken or relay.token event) — pair.ok exp 1797037747 relay https://orrery-relay.jinvu.workers.dev +19.9s
PASS [10] [3] relay: IK + hello → welcome — {"locale":"vi","machine":"bV8r9NRPUhRYaRpcf2yvFA","minProto":1,"name":"DESKTOP-SMCL7FD","ok":true,"proto":1,"re":"c1","r +20.8s
PASS [10] [3] relay: snapshot + ws.snapshot +20.9s
PASS [10] [4] relay: ws.open → paneKey — {"ok":true,"paneKey":"pmwcuo2oi6","re":"c2"} +21.1s
PASS [10] [5] relay: pty.attach ok — {"ok":true,"re":"c3","stream":1} +21.5s
PASS [10] [5] relay: typed echo + result come back — "\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-nnTh8W\\proj-b\"\u001b\\\r\nPS C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-nnTh8W\\proj-b> " +23.6s
PASS [10] [9] relay: revoke closes the relay session with 4401 +24.1s
PASS [11] remoteEnable(false) → enabled false, listening false — {"enabled":false,"listening":false,"relay":"off"} +24.1s
PASS [11] netstat: nothing listens on 4330 +25.0s
PASS [11] status relay off — off +25.0s
NOTE keepAwake setting after off: true (OS power request không kiểm được: powercfg /requests cần admin)
```
