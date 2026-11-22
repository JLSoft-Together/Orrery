# WP-18 RESULT — 2026-11-22T14:22:50.234Z (75s)

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
| 10 | Lặp 3–5 qua relay + thu hồi qua relay | LỖI | 1/4 |
| 11 | Tắt remote → listener đóng | ĐẠT | 3/3 |
| 12 | F3 ws.patch ≤ 1 s + log lệnh + tên nền tảng | LỖI | 4/5 |

## Log

```
PASS [1] remoteEnable(true) → enabled + listening — {"enabled":true,"listening":true,"port":4330,"relay":"off"} +11.8s
PASS [1] port = 4330 — 4330 +11.8s
PASS [1] addresses non-empty, all private IPv4 — ["192.168.0.106","192.168.0.235"] +11.8s
PASS [1] netstat: listener only on a private LAN IP (not 0.0.0.0 / 127.0.0.1 / ::) — 192.168.0.106:4330, 192.168.0.235:4330 +11.9s
PASS [1] netstat: every listener address is in status.addresses — 192.168.0.106:4330, 192.168.0.235:4330 +11.9s
PASS [2] remotePairStart → QR text — eyJ2IjoyLCJtYWNoaW5lIjoibkF3alIxSjRsbFJW +11.9s
PASS [2] phone_cli pair → pair.ok (native confirm answered ok by CCM_E2E_CONFIRM) — {"deviceId":"e2e-af0258755154","ev":"paired","machine":"nAwjR1J4llRVNTu7ZelyUQ","reply":{"machine":"nAwjR1J4llRVNTu7ZelyUQ","name":"DOANVU","t":"pair.ok"}} +12.0s
PASS [2] status.devices has the new device — ["E2E Phone One"] +12.0s
NOTE e2e-confirms.log: Ghép qua LAN từ IP 192.168.0.106: điện thoại android «E2E Phone One»? Thiết bị này sẽ xem các phiên và điều khiển Orrery trên máy này. Thu hồi bất cứ lúc nào trong Cài đặt → Remote.
PASS [3] IK session established (phone_cli ready) +13.6s
PASS [3] hello → welcome ok (machine, version, minProto) — {"chat":1,"control":true,"locale":"vi","machine":"nAwjR1J4llRVNTu7ZelyUQ","minProto":1,"name":"DOANVU","ok":true,"os":"windows","proto":1,"re":"c1","theme":{"mo +13.6s
PASS [3] event snapshot (protocolVersion + terminals) — seq 3 +13.6s
PASS [3] event ws.snapshot — rev 1795357307 tabs 1 +13.8s
PASS [3] ws.snapshot pane keys match the Pinia workspace store — phone ["pmwrxdvk30"] store ["pmwrxdvk30"] +13.8s
NOTE relay.token event: không có
PASS [4] ws.open → ok + paneKey — {"ok":true,"paneKey":"pmwrxdzpz2","re":"c2"} +16.9s
PASS [4] new pane is in the store +16.9s
PASS [4] ui.view unchanged — workspace → workspace +16.9s
PASS [4] active pane / tab unchanged — pmwrxdvk30 → pmwrxdvk30 +16.9s
PASS [4] new pane runs a shell in the asked cwd — {"state":"running","cwd":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-NWYVpw\\proj-a"} +16.9s
NOTE F3: ws.open gửi → pane mới có trong summary điện thoại sau 296 ms (qua ws.patch)
PASS [4] phone sees the new pane (ws.get) with a sessionId — 0 ms {"agent":"shell","busy":false,"cwd":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-NWYVpw\\proj-a","dirty":false,"key":"pmwrxdzpz2","kind":"term","preset" +17.1s
PASS [5] pty.attach → ok — {"ok":true,"re":"c3","stream":1} +17.2s
PASS [5] event pty.ready {cols, rows} +17.2s
PASS [5] output contains the typed echo and its result (≥ 2 × orrery-e2e) — "\\x5cproj-a\u0007\u001b]633;A\u0007\u001b]9;9;\"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-NWYVpw\\proj-a\"\u001b\\PS C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-NWYVpw\\proj-a> \u001b]633;B\u0007" +19.0s
PASS [5] desktop pane still running +19.0s
PASS [5] desktop view still unchanged +19.0s
PASS [6] ws.open while desktop is on Monitor → ok + paneKey — {"ok":true,"paneKey":"pmwrxe20p4","re":"c4"} +19.9s
PASS [6] pane reaches state running without opening Workspace — {"state":"running","cwd":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-NWYVpw\\proj-b"} +19.9s
PASS [6] ui.view still monitor +19.9s
PASS [7] a long command typed from the phone marks the pane busy (ranCommand) [finding F1 if FAIL] — ranCommand=true attach {"ok":true,"re":"c5","stream":2} +23.7s
PASS [7] a command typed on the desktop marks the pane busy (control for the confirm test) — ranCommand=true +24.5s
PASS [7] ws.close busy pane → ok + confirm cid (no immediate close) — {"confirm":"4522d88cef51e8688bba6f60","ok":true,"re":"c6"} +24.6s
PASS [7] event confirm.ask reaches the phone (cid, title, ok, cancel) — {"cancel":"Huỷ","cid":"4522d88cef51e8688bba6f60","detail":"Agent hoặc lệnh ở đó vẫn đang chạy và sẽ dừng.","ok":"Đóng","re":"c6","t":"confirm.ask","title":"Đóng proj-b?"} +24.6s
PASS [7] desktop shows no [role=alertdialog] +24.6s
PASS [7] pane still open while waiting +24.6s
PASS [7] confirm.answer ok → reply ok — {"ok":true,"re":"c7"} +24.7s
PASS [7] pane closed on the desktop +24.7s
PASS [12] F3: pane mới tới điện thoại ≤ 1 s sau ws.open, qua ws.patch (không cần ws.get) — 296 ms ws.patch +24.7s
NOTE log lệnh: ["đóng proj-b","mở proj-b","mở proj-a"]
PASS [12] log có dòng mở pane (ws.open) — ["đóng proj-b","mở proj-b","mở proj-a"] +24.7s
PASS [12] log có dòng đóng pane (ws.close) — ["đóng proj-b","mở proj-b","mở proj-a"] +24.7s
PASS [12] Settings hiện log lệnh (data-remote-log có ≥ 2 dòng) +25.9s
FAIL [12] F2: hàng thiết bị hiện "Không rõ" thay vì key thô — E2E Phone One
Worktree +25.9s
PASS [8] fs.list in pane cwd → entry hello.txt — {"data":[{"directory":false,"name":"hello.txt","outOfRoot":false,"path":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-NWYVpw\\proj-a\\hello.txt"}],"ok":true,"re":"c8"} +26.3s
PASS [8] fs.read inside root → text — {"data":{"bom":false,"eol":"lf","kind":"text","lang":"txt","mtimeMs":1795357294923.286,"path":"C:\\Users\\dauph\\AppData\\Local\\Temp\\ccm-e2e-v97-NWYVpw\\proj-a\\hello.txt","size":17,"text":"orrery e +26.3s
PASS [8] fs.read outside every root → denied — {"error":"denied","ok":false,"re":"c10"} +26.4s
PASS [8] fs.read UNC → invalid — {"error":"invalid","ok":false,"re":"c11"} +26.5s
PASS [8] fs.read with .. out of the root → denied or invalid — {"error":"denied","ok":false,"re":"c12"} +26.5s
PASS [9] remoteRevoke(deviceId) → true +26.5s
PASS [9] phone gets device.revoked +26.5s
PASS [9] LAN session closed with 4401 — {"code":4401,"ev":"closed","stage":"session","at":1795357321463} +26.6s
NOTE LAN reconnect: không vào được phiên ({"code":4401,"ev":"closed","stage":"msg2","at":1795357321565})
PASS [9] reconnect with the revoked key refused with 4401 — {"code":4401,"ev":"closed","stage":"msg2","at":1795357321565} +26.7s
PASS [9] device gone from status.devices +26.7s
NOTE relay status before item 10: off
FAIL [10] status.relay = online — off +71.9s
PASS [10] second phone paired over LAN — {"deviceId":"e2e-05f7e1a3f84e","ev":"paired","machine":"nAwjR1J4llRVNTu7ZelyUQ","reply":{"machine":"nAwjR1J4llRVNTu7ZelyUQ","name":"DOANVU","t":"pair.ok"}} +71.9s
FAIL [10] relay token available (pair.ok relayToken or relay.token event) — ["ready","json","push.config","snapshot","ws.snapshot","ws.patch","events","events","events","events","events"] +74.2s
FAIL [10] relay path skipped (relay offline / no token / no machine) — {"online":false,"token":false,"machine":"nAwjR1J4llRVNTu7ZelyUQ"} +74.2s
PASS [11] remoteEnable(false) → enabled false, listening false — {"enabled":false,"listening":false,"relay":"off"} +74.2s
PASS [11] netstat: nothing listens on 4330 +75.1s
PASS [11] status relay off — off +75.1s
NOTE keepAwake setting after off: true (OS power request không kiểm được: powercfg /requests cần admin)
```
