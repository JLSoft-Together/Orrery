# Báo cáo WP-v87-5-S11d — `/compact typed into the input, not submitted`

## Kết luận

Lỗi **không do `CCM_CORE=rust`**. Rust serve và Node collector chạy giống nhau ở bước này; fail phụ thuộc vào **cách mở E2E** (shell cha) và lịch sử PSReadLine của máy. Không sửa code Rust / `core-serve-host`.

## Nguyên nhân

1. Pane `shell` trên Windows luôn là Windows PowerShell 5.1 (`presets.ts`, `powershell.exe` đường dẫn tuyệt đối), nạp PSReadLine theo `PSModulePath` thừa kế từ process cha.
2. Khi E2E được chạy từ **pwsh 7** (PowerShell tool / Codex), `PSModulePath` có thêm `Documents\PowerShell\Modules`, `C:\Program Files\PowerShell\Modules`, `c:\program files\powershell\7\Modules`. pwsh chỉ tự bỏ các path này khi chính nó mở `powershell.exe`; qua chuỗi node → electron → pty thì chúng còn nguyên, nên 5.1 nạp PSReadLine 2.3 của pwsh 7, có **prediction từ history** (chữ xám nghiêng). Chạy từ Git Bash thì 5.1 chỉ thấy PSReadLine 2.0.0 (không có prediction).
3. Cuối mỗi lần chạy `v51-compact`, bước "Cancel compact" gõ handoff prompt kèm Enter lên dòng còn `/compact` + resume line, nên dòng `/compactRead `…` and continue.Prepare for /compact: …` vào `ConsoleHost_history.txt`.
4. Lần chạy sau, gõ `/compact` → PSReadLine hiện phần đoán của dòng history đó ngay sau con trỏ. Mission Control `textContent` gồm cả chữ đoán, nên regex `/\/compact\s*$/` fail. `/compact` vẫn được gõ đúng và **không** bị submit (check resume line sau đó PASS, ảnh chụp cho thấy chữ đoán xám).

Bằng chứng: ảnh `v51-03-compact-typed.png` của S11c — lượt rust 17:36 hiện chữ đoán từ lượt node 17:14 (`pPpmPw`), lượt rust 17:58 hiện từ lượt rust 17:36 (`TA9peI`). Lượt node 17:14 PASS chỉ vì là lượt pwsh đầu tiên sau khi history chưa có dòng `/compact…` (node lượt 1 dừng ở `v49-open`).

## Kết quả E2E `v51-compact` (port 4399, data tạm của `cdp.cjs`)

| Shell cha | `PSModulePath` | CCM_CORE | Kết quả check `/compact typed…` |
|---|---|---|---|
| Git Bash | mặc định | rust | PASS 4/4 |
| Git Bash (`v49-open` → `v50-groups` → `v51-compact`) | mặc định | rust | PASS |
| pwsh 7 | thừa kế pwsh 7 | node | **FAIL** (chữ đoán `xjkXwO`) |
| pwsh 7 | thừa kế pwsh 7 | rust | **FAIL** |
| pwsh 7 | bỏ các path pwsh 7 | rust | PASS |
| pwsh 7 | bỏ các path pwsh 7 | node | PASS |

Mọi check khác của `v51-compact` PASS ở mọi lượt.

## File sửa

Không có. Không có khác biệt Rust ↔ Node để sửa; test không bị nới.

## Đề xuất (cần main / chủ nhân chốt)

- **Cách chạy E2E**: chạy từ Git Bash, hoặc từ pwsh thì bỏ các path pwsh 7 trước khi chạy:
  `$env:PSModulePath = (($env:PSModulePath -split ';') | ? { $_ -notmatch '\\PowerShell\\(7\\)?Modules$' -or $_ -match 'WindowsPowerShell' }) -join ';'`
- **Sửa sản phẩm (tùy chọn)**: khi spawn Windows PowerShell 5.1 cho pane, bỏ các entry `PSModulePath` của pwsh 7 (như pwsh tự làm khi mở `powershell.exe`). Với app mở từ Explorer thì không đổi gì; khi app mở từ pwsh (`npm run desktop`) pane không nạp nhầm module của pwsh 7. Lưu ý: máy có PSReadLine ≥ 2.1 cài cho 5.1 vẫn sẽ có chữ đoán, nên check E2E này vẫn phụ thuộc history của máy.
- Phụ: mỗi lần chạy `v51-compact` thêm 1 dòng `/compact…Prepare for /compact…` vào history PowerShell thật của user.

STATUS: DONE
