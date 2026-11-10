# Báo cáo WP-v87-4-T4w

## Kết quả

- Build `node apps/desktop/build.mjs`: pass. `cargo build --release --locked -j 4 -p core-bin`: pass (38,36 giây). Web dist có sẵn, không chạy lại web build.
- Parity `npm test -w @ccm/collector -- --maxWorkers=4`: 467 pass, 3 skip; 42 test file pass, 1 skip.
- CPU demo 30×6, seed 8704, stage, settle 20 giây, đo 120 giây. Trung bình collector + tất cả PID `orrery-core` là **6,54%**, đạt ngưỡng **≤ 7,6%**. Node đối chứng trung bình 10,70%, Rust giảm 38,9%.
- So T3b Windows, Rust + usage trung bình 6,40%; T4 cao hơn 0,14 điểm phần trăm, gần ngang nhau. Chi tiết từng lượt, CPU-seconds và PID nằm ở `docs/perf/v87-step4-T4-win32.json`.
- E2E full: 56 bộ, 1365 assertion pass, 3 assertion fail thuộc 2 bộ. Rerun từng bộ fail riêng và so sánh Node hoàn tất.

| Mode / lượt | Collector avg / p95 | `orrery-core` CPU-s | Collector + core | RSS collector |
|---|---:|---:|---:|---:|
| Rust 1 | 4,8% / 8,4% | 2,4 | 6,75% | 98,5 MB |
| Rust 2 | 4,5% / 8,3% | 2,2 | 6,33% | 97,6 MB |
| Node 1 | 9,9% / 15,7% | 0,1 | 10,08% | 120,9 MB |
| Node 2 | 11,4% / 19,7% | 0,1 | 11,33% | 110,2 MB |

Cách tính ngưỡng CPU: `(collector cpuSec + tổng cpuSec của mọi PID orrery-core) / 120 × 100`. Máy: Windows 11 Pro 10.0.26200, Intel Core i7-9700, 8 logical CPU, RAM 47,9 GB. Tổng stage CPU dao động 26,2–79,3% (một core); phép đo chỉ tính collector và `orrery-core`.

## E2E fail và phân loại

| Nhóm | Bộ / kết quả | Bằng chứng |
|---|---|---|
| Lỗi core Rust | `v70-tools`: `response body is shown` fail trong full run và rerun Rust; pass khi đặt `CCM_CORE_PTY=node CCM_CORE_TRANSCRIPT=node CCM_CORE_USAGE=node`. Đây là lỗi do core Rust theo so sánh mode. | `apps/desktop/e2e/v70-tools.cjs:187`; log full và retry ở `%TEMP%\v87-step4-T4-win-e2e.log`, `%TEMP%\v87-step4-T4-v70-retry-rust.log`, `%TEMP%\v87-step4-T4-v70-retry-node.log` |
| Flaky | Không còn fail nào được phân loại flaky. `p2` pass trong full run và rerun riêng. | `%TEMP%\v87-step4-T4-win-e2e.log` và `%TEMP%\v87-step4-T4-p2-retry-rust.log` |
| Script/test cũ | `v51-compact`: 2 assertion fail về `/compact` submit và resume line prefill; rerun vẫn fail cả Rust mặc định lẫn khi ép PTY/transcript/usage sang Node. Không phụ thuộc core Rust. | `apps/desktop/e2e/v51-compact.cjs:99`, `:108`; log `%TEMP%\v87-step4-T4-win-e2e.log`, `%TEMP%\v87-step4-T4-v51-retry-rust.log`, `%TEMP%\v87-step4-T4-v51-retry-node.log` |

`p2` có dòng `PASS closing panes kills processes` dù phần mô tả ghi PID còn sống; assertion pass, không được tính là E2E fail.

## Trạng thái

Không sửa code sản phẩm. Chỉ thêm perf JSON và báo cáo này. Lock `%TEMP%\orrery-bench.lock` đã được gỡ sau khi build, đo và E2E kết thúc.

STATUS: PARTIAL


