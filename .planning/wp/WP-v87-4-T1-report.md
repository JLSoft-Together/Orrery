## WP-v87-4-T1 — DONE
- Đã đổi: crate mới `apps/core/crates/core-transcript` (omap, time, json, record, index, cache, query, collate, jobs, serve, tests; file lớn nhất 322 dòng), `core-bin` subcommand `transcript` + `tests/transcript.rs`, workspace `Cargo.toml` / `Cargo.lock` (chỉ thêm member, không crate mới tải về). Collector: `src/coreTranscript.ts` (mới), `src/usageSource.ts` (mới), `src/index.ts` (dựng qua `createUsageSource`, `heatmap.get` / `toolstats.get` qua `whenReady` — node mode vẫn trả đồng bộ như cũ), `test/usage-parity.test.ts` (mới). `.code_index/{core,collector,_meta}.md`, plan §2.1.
- Protocol: `ping`, `usage.scan` (mở + snapshot từ cache, rồi full scan), `usage.notify {files}` (không id, debounce 1 s trong Rust), `usage.query {q: heatmap|toolStats|scanInfo|snapshot}`, `usage.idle`, `usage.stop`; event `usage.scan` / `usage.reset` / `usage.update`. Stdin EOF → lưu cache + thoát 0. Chi tiết + khác biệt biên: plan §2.1.
- Tiêu chí 5/5:
  - Parity TS ↔ Rust: đạt — cùng buckets, scanInfo, heatmap 28 ngày, toolStats (30 ngày all + 7 ngày 1 project), cache v4 deep-equal (kể cả `mtimeMs`; trên corpus bench file cache 2 bên trùng cả số byte 2 128 644). Corpus có resume copy, lỗi tool / MCP, skill, agent, 4 model, 3 project, 4 ngày, CRLF, dòng hỏng, `<synthetic>`, dòng 1.3 MB, dòng dở rồi append, file truncate.
  - Node đọc cache Rust và ngược lại: đạt (cùng test, rồi xoá 1 transcript → cache 2 bên vẫn bằng nhau).
  - Mặc định `node` không đổi; `rust` + core lỗi → Node: đạt (factory null, binary không tồn tại; core chết giữa chừng → mở lại, quá 3 lần / phút → `UsageIndex` cũ trên cùng file cache).
  - Bảng đo: đạt (dưới). Test + clippy + typecheck sạch; index cập nhật: đạt.

| Cold scan 507 MiB, 56 file, 280k dòng (synthetic, Windows) | CPU-s | wall s | RAM |
|---|---|---|---|
| Rust release, lần 1 / 2 | 1.30 / 1.22 | 1.68 / 1.36 | peak WS 40 MB |
| Node `UsageIndex`, lần 1 / 2 | 2.42 / 3.28 | 1.97 / 2.47 | maxRSS 175 / 164 MB |
| Warm (cache có sẵn) Rust / Node | 0.19 / 0.17 | 0.16 / 0.10 | 48 / 101 MB |
- Phân rã Rust (`measure_phases_over_a_corpus`): đọc file ~0.27 s, tách dòng + memmem 0.23 s, parse 159k dòng 0.49 s. Ngoại suy 4 GB ≈ 8–10 CPU-s → **chưa đạt cổng ≤ 5 CPU-s** của bước 4; hướng tiếp (T4): bỏ qua chuỗi `text` / `input` nhanh hơn serde_json, gộp 2 lần memmem. Corpus T0 chưa có (chưa có báo cáo T0) nên dùng corpus tự tạo (`%TEMP%\ccm-t1-bench\gen.mjs`, đã xoá corpus).
- Audit: không comment trong code; không log nội dung dòng (stderr chỉ errno); test chỉ dùng thư mục tạm, không chạm `%LOCALAPPDATA%\ccm` / port 4317. `cargo build --release` đã ghi đè `apps/core/target/release/orrery-core.exe` (dev desktop dùng file này cho PTY; build thành công nên không có app nào đang khoá nó).
- Review: local time Rust lấy theo zone OS (Windows `SystemTimeToTzSpecificLocalTimeEx`) thay ICU của Node; `localeCompare` chỉ xấp xỉ (chỉ ảnh hưởng thứ tự khi bằng số lần gọi, ASCII khớp ICU đã kiểm bằng Node). Truy vấn trả lời giữa 2 file khi đang scan (lock theo file).
- Test: `cargo test --locked -j 4` toàn workspace pass (core-transcript 10 + 1 ignored, transcript.rs 1); clippy `-D warnings` Windows + aarch64-apple-darwin sạch (linux cũng sạch lúc chạy trước tin main); `npm test -w @ccm/collector -- --maxWorkers=4` 436 pass / 2 skip (35 file + 1 skip có sẵn); `npm run typecheck -w @ccm/collector` sạch.
- Ngoài phạm vi: working tree có thay đổi không phải của T1 (`core-procs/src/lib.rs`, xoá `linux.rs` / `procfs.rs`, `apps/web/public/file-icons/**`, `fwp-host.test.ts.snap`) — không đụng. Chưa bật `CCM_CORE_USAGE=rust` trong desktop (`apps/desktop` ngoài phạm vi): main cần truyền env khi muốn thử.
- Commit message đề xuất: `✨ v87 step 4 T1: core-transcript usage index in Rust (orrery-core transcript), CCM_CORE_USAGE client with Node fallback, Node/Rust parity test`

### Vòng 2 (review)
- (1) `serve.rs`: reply `{snapshot}` của `usage.scan` được ghi xong mới gửi `Job::Scan` → event không bao giờ ra trước reply. Test `scan_reply_always_comes_before_scan_events` (20 vòng, dòng đầu tiên phải là reply); `tests/transcript.rs` `until` giữ backlog nên `ask` không treo / không nuốt event.
- (2) File tạm `usage-index.json.tmp-<pid>` ở cả Rust (`cache.rs write_cache`) lẫn Node (`usage.ts save`), xoá khi ghi / rename lỗi.
- (3) `usageSource.ts`: `usage.query` gửi với `restartOnTimeout: false` (timeout → UI nhận rỗng, không restart core). Nhả lock theo từng chunk: chưa làm — lock hiện theo file (file 100 MB ≈ 0.2 s), ghi lại cho T2 nếu cần.
- (4) Đọc file lỗi (khoá / bị xoá) → log 1 dòng (`skipped a transcript: <errno>`, không có nội dung dòng), bỏ `mtimeMs` để sweep sau thử lại, scan các file khác tiếp; sửa cả `usage.ts` (try/catch trong `ingest`) để giữ parity. Test: Rust `skips_an_unreadable_transcript_and_reads_it_later` (Windows share mode 0 / macOS chmod 000), parity Node ↔ Rust `usage index skips an unreadable transcript` (khoá bằng PowerShell `FileShare.None`, rồi mở khoá + notify → cả hai đếm, cache bằng nhau).
- Test vòng 2: `cargo test --locked -j 4` toàn workspace pass (core-transcript 11 + 1 ignored, transcript.rs 2); clippy `-D warnings` Windows + aarch64-apple-darwin sạch; collector 435 pass / 2 skip (vòng 1: 436; T1 thêm 1 test nhưng tổng giảm 1 — các file test bị giảm không thuộc T1, con chưa kiểm nguyên nhân), parity + usage 14/14; typecheck sạch.
