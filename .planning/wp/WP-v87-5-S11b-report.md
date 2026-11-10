# WP-v87-5-S11b — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S11b.md`. Không commit / push / checkout, không sửa `apps/collector/src`, `packages/shared`, `apps/web`, `apps/desktop/src`. Mọi lần chạy `serve` / collector: `CCM_PORT` trống (≠ 4317), data / Claude / Codex / Codewhale home tạm; không chạm app thật; chỉ kill pid tự spawn (sau cùng không còn `orrery-core` / `PING.EXE` mồ côi). CPU trước việc nặng 17–51 %, mỗi lúc 1 việc nặng.

## Nguyên nhân chi phí đọc 5–7 ms / file

**Windows Defender quét file mỗi lần mở lại sau khi file vừa được ghi.** Không phải stat thừa, không đọc lại từ đầu (offset vẫn giữ), không phải quét thư mục.

- Đếm trong `orrery-core serve` (probe tạm, đã gỡ), demo 30×6 tick 400 ms, ~75 s: `File::open` + `metadata` **8 260 lần, trung bình 5,06 ms = 41,8 s**; đọc phần thêm + parse 0,08 ms / lần; `read_dir` subagents 0,13 ms / lần; phần còn lại của `discover` là mở `meta.json` mới (file mới → cũng bị quét, không tránh được, Node cũng làm).
- Bench riêng (300 file đang được demo ghi, 12 s): mở lại + đọc file vừa đổi **p50 6,7 / p99 19 ms**; file không đổi 0,03 ms; `fs::metadata(path)` 0,03 ms (mở không quyền đọc, không bị quét); **đọc qua handle đang giữ 0,01 ms** (p99 0,08). "stat trước rồi mới mở" không đỡ được vì file cần đọc chính là file vừa đổi.

## Đã làm

1. `apps/core/crates/core-transcript/src/held.rs` (mới): `Held` (handle + birth time, + dev/ino trên unix), `open(path, &mut held, keep)`: có handle và `keep` → `fs::metadata(path)` so birth time + độ dài với `metadata` của handle; khớp → đọc bằng handle; khác / lỗi → bỏ handle, mở lại theo đường dẫn y như cũ (xoá → không đọc, thay / cắt ngắn → đi đúng nhánh `size < offset` cũ). Max 2 048 handle toàn process (`held_count`), vượt → mở / đóng như cũ.
2. `tail.rs`: `JsonlTailer.keep` (mặc định `false` → mọi chỗ dùng tailer khác không đổi), `holds()`, `release()`; `read_some` → `held::open` + `read_from` (thân đọc cũ, không đổi).
3. `tracker/`: `Tracker.hold` (mặc định `keep_default()` = chỉ Windows), tailer main + subagent tạo với `hold`; subagent resolve → `keep = false` + trả handle (đọc lại về sau mở / đóng như cũ); `Tracker::release` + `held_files`; `reset_session` đóng theo `Drop`. `tracker/serve.rs` (op `tracker.*` cho Node collector) tắt `hold` → bản tham chiếu Node giữ nguyên.
4. `core-monitor/src/tracker.rs`: `MonTracker::mark_ended` → `core.release()` (terminal đã end còn nằm 120 phút không giữ file). `core-serve/src/stats.rs`: thêm `heldFiles` vào `serve.stats`.
5. `scripts/trace-lockstats.mjs` mở rộng: `--server=rust|node|both`, `--node-core=rust|node`, demo **không seed** (timestamp thật) để đo **ghi → WS** = `at` của `activity` `tool.started` / `tool.failed` (timestamp dòng demo ghi) → lúc nhận frame; p50 / p90 / p99 / max; bỏ 2 s đầu sau khi nối WS (`--settle`, lúc nối snapshot làm trễ ~1,5 s mọi event trong cửa đó — đo `--late` thấy toàn bộ event > 800 ms nằm trong 1 s đầu), `--late=ms` liệt kê event trễ, `--seed` vẫn dùng được.
6. Không làm đọc song song: sau khi giữ handle `outsideLockMs` p99 ~28 ms, đã dưới cổng 100 ms nhiều; thêm luồng chỉ thêm rủi ro (sensitive check dùng chung).
7. Docs: plan §3.11 mới, `.code_index/core.md` (tail `keep` / `held.rs`, Tracker held handles, lệnh `trace-lockstats`). Chưa ghi `.code_index/_meta.md` (ngoài phạm vi brief) — main thêm 1 dòng nếu muốn.

## File

Mới: `apps/core/crates/core-transcript/src/held.rs`, `apps/core/crates/core-transcript/src/heldtests.rs`, báo cáo này.
Sửa: `apps/core/crates/core-transcript/src/{lib,tail}.rs`, `apps/core/crates/core-transcript/src/tracker/{mod,serve,subs}.rs`, `apps/core/crates/core-monitor/src/tracker.rs`, `apps/core/crates/core-serve/src/stats.rs`, `scripts/trace-lockstats.mjs`, `.code_index/core.md`, `.planning/PLAN-v87-step5-core-store.md` (§3.11).

## Test (số thật)

- `cargo clippy --locked -j 4 --workspace --all-targets -- -D warnings`: 0 cảnh báo.
- `cargo test --locked -j 4 --workspace`: **460 pass, 0 fail, 6 ignored** (S11a 456 + 4 mới `heldtests`: handle giữ đọc ra đúng như mở lại qua append / dòng dở / ghi đè ngắn hơn / release; file bị xoá rồi tạo lại → mở lại theo đường dẫn; `keep` tắt không giữ; tracker giữ main + subagent đang chạy, subagent resolve → trả handle, `release` → 0).
- `npm test -w @ccm/collector -- --maxWorkers=4`: 52 file pass / 1 skip, **537 pass / 3 skip** (twin monitor / agents / toast / sources / persist + race twin).

## Độ trễ (`node scripts/trace-lockstats.mjs`, demo 30×6 tick 400 ms 60 s, release, máy rảnh 17–51 % CPU)

Ghi → WS (ms, `tool.started` / `tool.failed`, ~4 600–6 200 event mỗi lượt):

| | p50 | p90 | p99 | max | `outsideLockMs` p99 / max | lock trong tick p99 / max |
|---|---|---|---|---|---|---|
| Rust **trước** (HEAD S11a) | 593 | 1 037 | 2 198 | 3 688 | 508 / 1 078 | 3,7 / 7,4 |
| Node trước (collector + transcript Rust, mặc định) | 496 | 1 828 | 6 831 | 57 821 | — | — |
| Rust **sau** lượt 1 | 200 | 257 | 305 | 351 | 27 / 260 | 1,8 / 5,4 |
| Rust sau lượt 2 | 202 | 261 | 314 | 342 | 29 / 231 | 1,9 / 4,6 |
| Node sau (collector + transcript Rust) | 407 | 1 714 | 4 217 | 55 933 | — | — |
| Node thuần (`CCM_CORE_*=node`) | 170 | 214 | 615 | 1 022 | — | — |
| Cùng lúc `--server=both`: Node / Rust | 359 / 201 | 1 897 / 256 | 4 130 / **293** | 54 602 / 308 | Rust 29 / 115 | 1,9 / 5,3 |

- "Trước" đo bằng bản script đầu của gói này (chưa bỏ 2 s đầu); 2 lượt trước khác chỉ ở chỗ đó, phần đuôi trước là do vòng đọc dài suốt lượt (`outsideLockMs` p99 508), không phải cửa nối WS.
- Cổng: Rust p99 **≤ Node p99** (305 / 314 / 293 ms ≤ 615 ms Node thuần và ≤ 4 217 ms Node mặc định) ✓; `outsideLockMs` p99 **27–29 ms ≤ 100** ✓; lock trong tick p99 **1,8–1,9 ms ≤ 20** ✓. `/health` p99 2,4–2,5 ms. `heldFiles` lúc tắt 65–82.
- Sàn p50 ~200 ms = debounce file project 150 ms + batch 50 ms (Node thuần 170 ms).
- Node mặc định có đuôi rất dài (max ~55 s, cả trước lẫn sau, không do gói này): Node collector + `orrery-core transcript` tuần tự, transcript Rust trên đường Node vẫn mở lại file mỗi lần (đã cố ý tắt `hold` cho đường này để giữ bản tham chiếu). Bật `hold` cho `tracker.*` sẽ đỡ cho người dùng `CCM_CORE=node` — việc sau nếu main muốn.

## Trace `--server=both` (demo 30×6 seed 8704, mỗi lượt ~175 s)

| Lượt | Frame Node / Rust | Fold | `--strict` |
|---|---|---|---|
| 1 | 833 / 724 | **0** | 50+ |
| 2 | 826 / 755 | **0** | 50+ |
| 3 | 825 / 770 | **0** | 50+ |

`--strict` (so chuỗi event phẳng theo thứ tự) theo plan §3.1 mục 7 chỉ dùng cho kịch bản trong process với thời gian giả; trên trace demo sống thứ tự / gộp event giữa 2 process chạy độc lập phụ thuộc nhịp đọc nên luôn khác (S10 / S11a cũng chỉ báo fold). Khác biệt strict lượt 1 là cùng một cập nhật agent tách ở hai patch khác chỗ (`status` / `completedAt` lệch 1 vị trí), fold cuối mỗi bước vẫn bằng nhau. Parity theo thứ tự đúng nghĩa (strict + fold 0) vẫn giữ qua twin trong process (`npm test -w @ccm/collector`: twin monitor / agents / race, pass).

## Rủi ro / lưu ý

- Windows: giữ handle (share read / write / delete của Rust std) không chặn Claude Code ghi / xoá file; xoá thư mục project khi monitor còn giữ handle chạy được nhờ xoá kiểu POSIX của NTFS (Win10 1809+). Handle được trả khi subagent xong, terminal end, đổi session.
- Ca hiếm không phát hiện được: file bị xoá rồi tạo lại cùng tên trong 15 s (NTFS "tunneling" giữ birth time) **và** cùng độ dài → đọc tiếp file cũ cho tới lần ghi kế (độ dài khác → mở lại). Transcript Claude Code chỉ append nên không gặp.
- macOS: không giữ handle (`keep_default` chỉ Windows) → hành vi y như trước; không đụng giới hạn fd.
- Chưa đo trên máy macOS / fixture thật 27 MB; việc S11 khác (E2E `CCM_CORE=rust`, RAM / CPU, đổi mặc định) chưa thuộc gói này.

STATUS: DONE
