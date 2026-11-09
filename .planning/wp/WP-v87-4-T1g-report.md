# WP-v87-4-T1g — kết quả đo usage scan cold 4.29 GB

Máy Windows 11 Pro, Intel Core i7-9700 (8 core), Node v24.15.0. Corpus có 192 file JSONL, tổng 4,294,705,152 byte; mỗi dòng 4,096 byte, một record có `usage` trên 100 record. Dùng executable release đã có; không cần build. Mỗi cold run dùng cacheFile mới; warm run tái sử dụng cache của cold run đầu tiên. Tất cả lượt scan xử lý đủ 192 file và về trạng thái `ready`.

| Backend | Loại | Lượt | CPU-s user + system | Wall | Peak RSS |
|---|---|---:|---:|---:|---:|
| Node `UsageIndex` | cold | 1 | 7.98 | 6.240s | 128 MB |
| Node `UsageIndex` | cold | 2 | 6.70 | 4.181s | 151 MB |
| Node `UsageIndex` | cold | 3 | 6.36 | 3.758s | 157 MB |
| Node `UsageIndex` | **cold median** | — | **6.70** | **4.181s** | **151 MB** |
| Node `UsageIndex` | warm | 1 | 0.05 | 62ms | 138 MB |
| Rust `orrery-core transcript` | cold | 1 | 2.17 | 4.965s | 8 MB |
| Rust `orrery-core transcript` | cold | 2 | 1.88 | 1.944s | 8 MB |
| Rust `orrery-core transcript` | cold | 3 | 2.84 | 3.861s | 8 MB |
| Rust `orrery-core transcript` | **cold median** | — | **2.17** | **3.861s** | **8 MB** |
| Rust `orrery-core transcript` | warm | 1 | 0.02 | 62ms | 9 MB |

**Kết luận:** Rust đạt cổng cold usage scan **≤ 5 CPU-s**: trung vị 2.17 CPU-s, nhanh hơn Node khoảng 3.09 lần theo CPU-s. Cả ba lượt Rust đều dưới 5 CPU-s. Node chưa đạt cổng, trung vị 6.70 CPU-s.

Cách đo: Node lấy CPU user + system bằng `process.cpuUsage()`, RSS lấy mẫu mỗi 25ms; Rust đo CPU tổng của process con bằng Windows `Get-Process.CPU` (user + system), RSS bằng `PeakWorkingSet64`. Wall Rust tính từ lúc spawn process đến event `usage.reset`; Node tính từ `UsageIndex.start()` đến khi scan ready và idle. Runner được lưu trong `%TEMP%`, không thuộc repo. Không chạm app Orrery đang chạy hoặc data dir người dùng.

Kết quả máy và từng lượt: `docs/perf/v87-step4-T1-cold-win32.json`.
STATUS: DONE
