## WP-v87-4-T0 — DONE
- `demo-fixture.mjs`: thêm `--seed` và `--record`; cùng seed `8704` cho 410 file transcript giống hệt SHA-256 qua 2 lần chạy. Không truyền cờ vẫn dùng luồng random / thời gian cũ.
- `perf-bench.mjs`: thêm `--collector-prof`, `--collector-prof-seconds`, `--seed`; profiler dùng Node Inspector trên collector nguồn chạy riêng sau lượt CPU để không làm nhiễu số baseline. `NODE_OPTIONS=--cpu-prof` làm Electron thoát mã 1 nên không dùng cách đó.
- Baseline: Windows, Node v24.15.0, Orrery 3.4.0, stage, 8 core; demo 30×6, 20s settle + 120s đo, 34 ticks.

| Phép đo | CPU avg | CPU p95 | CPU-s | Wall | Peak RSS |
|---|---:|---:|---:|---:|---:|
| Collector baseline | 10.3% | 16.7% | 8.6 | 120s | 107.9 MB |
| Usage cold, 4.2947 GB, lượt 1 | — | — | 5.06 | 3.32s | 135 MB |
| Usage cold, lượt lặp | — | — | 7.02 | 6.08s | 123 MB |
| Usage warm, lượt 1 / lặp | — | — | 0.11 / 0.05 | 61 / 75ms | 81 / 92 MB |

- Corpus tổng hợp có 192 JSONL, dòng 4 KiB, 1% assistant record có `usage`; giữ tại `%TEMP%\orrery-usage-corpus-23300` để chạy parity. `UsageIndex` đạt `ready` cả cold và warm.
- Top 20 hàm self-time đã ghi trong JSON. CPU collector hiện cao hơn mục tiêu ~7.6%; cold scan biến thiên 5.06–7.02 CPU-s so với mốc ≤5, cần tính đến khi đánh giá T4.
- Đã chạy `node --check` cho 3 script, hash parity, smoke benchmark và hai scan 4 GB. Không chạy build; stage sẵn có ngày 09/11. Không chạm app Orrery đang chạy, port 4317 hay data dir người dùng.
- File kết quả: `docs/perf/v87-step4-baseline-win32.json`; index: `.code_index/_meta.md`.
STATUS: DONE
