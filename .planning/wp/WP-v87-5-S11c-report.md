# Báo cáo WP-v87-5-S11c

Phạm vi thực hiện: chỉ đo và chạy test; không sửa code trong `apps/` hoặc `packages/`. Orrery đang chạy trên port 4317 và dữ liệu `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm` không bị đụng tới. Mọi instance đo dùng port, data, userData và fixture riêng trong `%TEMP%`.

## Build chuẩn bị

| Lệnh | Kết quả |
|---|---|
| `npm run build -w @ccm/web` | PASS, Vite build xong trong 25.27 s; có warning về chunk lớn hơn 500 KB và import động/tĩnh. |
| `node apps/desktop/build.mjs` | PASS, build stage và các bundle main/gramjs/collector/preload/pty-host xong trong khoảng 0.8 s. |
| `cargo build --release --locked -j 4 -p core-bin` (`apps/core`) | PASS, Cargo báo xong release trong 0.36 s. |

## E2E đủ bộ

Runner `apps/desktop/e2e/run.cjs` chạy tuần tự 54 script. Số `PASS` dưới đây là số dòng PASS trong log; số fail là tổng mà runner đếm từ các dòng `FAIL`.

| CCM_CORE | Lượt | PASS | FAIL | Kết quả |
|---|---:|---:|---:|---|
| node | 1 | 617 | chưa tổng kết | Bị dừng thủ công tại `v49-open` khi phát hiện lỗi cleanup `EPERM` ở `tier2` và `project-panes`; không có kết quả đủ bộ. |
| node | 2 (retry đủ bộ) | 1366 | 3 | FAIL. Lỗi đầu tiên: `FAIL colour swatch — timeout: color.ts editor`. Các lỗi khác: `FAIL crashed — EPERM` khi dọn thư mục temp ở `v77b`; `FAIL Shift+Del forgets a closed project`. `v78` cũng báo `SOME FAILED (17 checks)`. |
| rust | 1 | 1367 | 1 | FAIL. Lỗi đầu tiên: `FAIL /compact typed into the input, not submitted`. |
| rust | 2 (retry đủ bộ) | 1364 | 4 | FAIL. Lỗi đầu tiên: `FAIL /compact typed into the input, not submitted`. Các lỗi khác: `markdown split — timeout: README.md editor`, `colour swatch — timeout: color.ts editor`, `crashed — EPERM` khi dọn thư mục temp ở `v77b`. Có lỗi cleanup `EPERM` tại `tier2`, `project-panes` và `v79-shots`; runner kết thúc với 4 fail. |

## RAM / CPU

Mỗi mode có 60 s ổn định và 24 khoảng lấy mẫu cách nhau 5 s. Working Set tổng được tính bằng tổng cây process tại mỗi mẫu. CPU là % tổng CPU máy theo 8 logical processor; process CPU được gom theo tên process, theo mẫu. Fixture: 10 sessions, 3 subagents. `CCM_VERIFY_PROCESSES=0`.

| Mode | Tổng Working Set trung bình | Tổng Working Set max | Process | Working Set TB / max (MB) | CPU TB / max (%) |
|---|---:|---:|---|---:|---:|
| node | 527.9 MB | 529.7 MB | `electron.exe` | 97.7 / 122.4 | 0.00 / 0.08 |
| node | 527.9 MB | 529.7 MB | `orrery-core.exe` | 5.7 / 7.4 | 0.00 / 0.04 |
| node | 527.9 MB | 529.7 MB | `conhost.exe` | 7.5 / 7.5 | 0.00 / 0.00 |
| rust | 413.1 MB | 429.3 MB | `electron.exe` | 98.6 / 123.6 | 0.00 / 0.12 |
| rust | 413.1 MB | 429.3 MB | `orrery-core.exe` | 11.4 / 11.7 | 0.02 / 0.08 |
| rust | 413.1 MB | 429.3 MB | `conhost.exe` | 7.5 / 7.5 | 0.00 / 0.00 |

BLOCKED: tổng CPU theo cây process không được script đo ghi lại ra file; chỉ còn số CPU đã tổng hợp theo tên process. Không ghi số tổng CPU khi không thể đối chiếu lại từ dữ liệu lưu.

## Độ trễ ghi → WS (`trace-lockstats --server=both`)

Cả hai lượt dùng 30 sessions, 6 subagents, tick 400 ms, thời lượng 60 s; `nodeCore=rust`.

| Lượt | Server | frames | writeToWs p50 / p90 / p99 / max (ms) | lockMs p50 / p99 / max (ms) | tableReadsUnderLock |
|---|---|---:|---|---|---:|
| 1 | node | 635 | 396 / 1929 / 4032 / 54175 | — | — |
| 1 | rust | 386 | 202 / 256 / 310 / 352 | 0.27 / 1.78 / 8.08 | 0 |
| 2 | node | 650 | 367 / 1761 / 3840 / 53820 | — | — |
| 2 | rust | 396 | 201 / 251 / 291 / 313 | 0.30 / 1.80 / 8.47 | 0 |

Node server không xuất `serve.stats`; ô lockMs và tableReadsUnderLock không có số đo.

## Trace capture + compare (fold)

| Lượt | Steps | Node frames / events | Rust frames / events | Thời lượng | Differences |
|---|---:|---:|---:|---:|---:|
| 1 | 16 | 828 / 23113 | 764 / 22989 | 175.5 s | 0 |
| 2 | 16 | 828 / 23061 | 774 / 22840 | 175.4 s | 0 |

`trace-compare.mjs` báo `0 differences` ở cả hai lượt.

## Tổng kết

Build và trace compare đạt. E2E còn fail ở cả hai mode; đo tổng CPU theo cây process không được lưu nên mục đó chưa đủ báo cáo.

STATUS: PARTIAL
