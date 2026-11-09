## WP-v87-4-T2 — DONE
- Đã đổi:
  - `apps/core/crates/core-transcript/src/` mới: `tail.rs` (JsonlTailer + `signal_line_filter` / `lite_filter`), `extract.rs` (`extractSignals`), `origin.rs` (origin / peer / handback / task-notification / `askOf` / `socket_key` / `decode_uri_component`), `rules.rs` (`touchOf` / `commandHits`), `conflict.rs` (`portConflictIn`), `state.rs` (`TranscriptState`), `spans.rs` (`ToolSpanLog`), `signal.rs` (`Signal` 20 loại + `to_json`), `jsstr.rs` (helper JS: trim, slice UTF-16, so sánh UTF-16, `toISOString`, số nguyên, lone surrogate), `sigserve.rs` (handle + op), `sigtests.rs`. File lớn nhất 323 dòng (`rules.rs`).
  - `serve.rs`: thêm `signal_op` chạy trước op usage (+16 dòng, op `usage.*` không đổi). `lib.rs` khai báo module. `Cargo.toml` + `Cargo.lock`: thêm `sha2 0.11` (deps `digest` / `cpufeatures` / `cfg-if` đã có trong lock, build offline).
  - `core-bin/tests/transcript.rs`: test `signals_open_read_close_next_to_usage`.
  - Collector: `test/transcript-parity.test.ts` + `test/transcript-parity-corpus.ts` (mới). Không đụng `apps/collector/src/**` (cả `coreTranscript.ts`: test dùng `createCoreTranscript().request` sẵn có, không cần hàm mới).
  - `.code_index/core.md` (mục "Signals layer"), `.code_index/collector.md` (dòng WP T2), plan §2.2 (protocol `signals.open|read|close`).
- Tiêu chí: 6/6.
  - Parity: 3 332 file, 51 168 dòng, 3 497 lần đọc, 153 214 signal, đủ 20 loại; signals + state + span đổi khớp sau MỖI lần đọc. Corpus: transcript dạng tracker (main, background 1.8 MB full + lite, message / peer / handback), resume copy, dòng biên (dòng dở cắt đôi từng dòng với chunk 7 / 64 / 1 MiB, JSON hỏng, BOM + CRLF, secret trong lệnh, reply 3 000+ ký tự + emoji, AskUserQuestion, limit hit / notice, compact, port conflict, lone surrogate), cắt ngắn file rồi ghi tiếp, demo 30×6 seed 8704.
  - Redaction: chỉ field whitelist (test Rust + test parity kiểm không có `SECRET` / `description`); lệnh chỉ ra rule + target như TS `ToolTouch`; layer mới không ghi stderr. `maskSecrets` không nằm trong đường signal của TS (chỉ prompt-search / sampler) nên Rust cũng không dùng — "secret bị mask" ở đây = nội dung lệnh không bao giờ rời hàm.
  - Tail: dòng dở, truncate, ghi tiếp, `fromEnd` / `maxLine` (test Rust + parity chunk nhỏ).
  - `usage.*` cũ: test T1 vẫn pass (`tests/transcript.rs` 3/3, usage-parity trong suite collector).
  - Bảng đo: dưới. Test + clippy + typecheck sạch; index + plan §2.2: đã cập nhật.
- Audit / Review:
  - Không comment trong code mới; không log nội dung dòng; test chỉ dùng thư mục tạm (`mkdtemp`, `%TEMP%\ccm-core-signals-*`), không chạm port 4317 / `%APPDATA%\Agent Monitor` / `%LOCALAPPDATA%\ccm`; release build để đo đặt ở `%TEMP%\ccm-t2-target` (không ghi đè `apps/core/target/release`), đã xoá cùng corpus bench; không còn process `orrery-core` sót.
  - Khác biệt biên đã ghi plan §2.2 + core.md: lone surrogate JS (astral bị cắt ở `slice(0, n)`, `\ud800` trong dòng) → Rust bỏ / U+FFFD (test parity bỏ lone surrogate phía TS trước khi so, case vẫn nằm trong corpus); `limit_hit.resetsAt` vượt khoảng Date → Rust bỏ field, TS throw `RangeError`; `Date.parse` dạng không ISO; JSON sâu > 128 / số vượt f64.
  - Rủi ro cho T3: `signals.read` chạy trên main thread nên đọc file lớn làm chậm reply `usage.query`; op không `id` vẫn chạy (open không id = handle không ai đóng). Tracker dùng 1 `ToolSpanLog` chung cho main + subagent của terminal, op hiện tạo 1 log / handle — T3 cần gom theo terminal.
- Test:
  - `cargo test --locked -j 4` (apps/core, PowerShell) → toàn workspace pass: core-transcript 27 pass + 2 ignored (đo), core-bin transcript 3/3, flow 8 (+2 ignored), procs 3 (+1), pty 16, spike 1, stub 4, core-procs 19, core-pty 35.
  - `cargo clippy --locked -j 4 --all-targets -- -D warnings` Windows + `--target aarch64-apple-darwin` → sạch.
  - `npm test -w @ccm/collector -- --maxWorkers=4` → 37 file pass / 1 skip; 442 pass / 3 skip / 0 fail (có parity mới 3 test; bench skip khi không có `CCM_T2_BENCH`).
  - `npm run typecheck -w @ccm/collector` → sạch.
- Đo (Windows, demo 30×6 seed 8704 `--tick=5 --duration=10 --peers=10`: 7 414 file, 62 MB, 428 557 signal, Rust release):

| Parse corpus (đọc + lọc + parse + state) | CPU-s lần 1 / 2 |
|---|---|
| TS (`JsonlTailer` + `signalLineFilter` + `extractSignals` + `TranscriptState`) | 3.81 / 3.88 |
| Rust qua op `signals.open/read/close` (CPU process con, gồm serialize JSON reply 33 MB) | 2.25 / 2.16 |
| Rust trong process (`measure_signals_over_a_dir`): đọc + parse / serialize JSON | 1.22 s / 0.24 s wall |

  Rust ~1.7× nhanh hơn tính cả reply; phần nặng là `serde_json::Value` cho cả dòng (gồm body tool_result). Hướng tăng tốc (T3 / T4): parse kiểu typed bỏ qua `text` / `content` như `record.rs` của T1, và không gửi `state` / `signals` thô khi T3 đổi ranh giới sang `TerminalSession` / `Agent[]`.
- Ngoài phạm vi cần main xử lý:
  - `.code_index/_meta.md` chưa thêm dòng (ngoài phạm vi brief); đề xuất: `- v87 step 4 T2: core-transcript signals layer (tail / filter / extractSignals / TranscriptState / ToolSpanLog / touchOf / portConflictIn) + op signals.open|read|close, parity test transcript-parity.test.ts; see core.md / collector.md.`
  - Working tree có thay đổi không phải của T2 (`apps/desktop/test/__snapshots__/fwp-host.test.ts.snap`, `.planning/RELEASE-v2.3.0.md`, `docs/qa/...png`) — không đụng.
- Commit message đề xuất: `✨ v87 step 4 T2: transcript signals in Rust (tailer, line filters, extractSignals, TranscriptState, ToolSpanLog, touchOf, portConflictIn), orrery-core transcript signals.open/read/close, Node/Rust parity test on tracker + demo corpus`

### Vòng 2 (review opus: 2 med + 5 low)
- (1 med) `signals.read {maxBytes}` (mặc định 4 MiB) trả `more: true` để caller đọc tiếp; `JsonlTailer::read_some(budget)` giữ size đích giữa các trang nên ghép các trang = 1 lần `readEach` TS (cả `size` cho lite filter). Test: Rust `budgeted_reads_add_up_to_one_full_read` (>10 trang, signals / spans / state bằng đọc 1 lần), core-bin `signals_need_an_id_cap_ids_and_page_by_bytes` (trang 4 KiB, `ping` trả lời giữa các trang), parity chạy thêm `maxBytes` 50 và 200 000 (426 trang thêm, vẫn khớp).
- (2 med) `jsonline.rs` mới: lồng > 128 → parse không giới hạn độ sâu (serde_json feature `unbounded_depth`, không có `serde_stacker` offline) trên thread stack 16 MiB + 2 KiB / ngoặc (max 1 GiB, vượt thì bỏ dòng), extract + drop value ngay trên thread đó; số ngoài khoảng f64 (`1e400`, `-1E999`, `-2e308`) → `-1`, mọi đường signal xử lý như JS ±Infinity. Corpus parity thêm 4 dòng (lồng 400 tầng, usage / compact / limit hit có `1e400`); test Rust `out_of_range_numbers_read_like_infinity`, `deep_lines_still_give_signals`.
- (3 low) `signal_op` kiểm `id` trước: dòng `signals.*` không id bị bỏ qua (không mở handle, không đọc mất signal); `signals.<lạ>` → `op`.
- (4 low) `toolUseIds` max 20 000, vượt → lỗi `toolUseIds` (không cắt ngầm); test.
- (5 low) `signals.open` `maxLine` mặc định 64 MiB.
- (6) Plan §2.2 ghi: phân trang, độ sâu / số lớn, `js_slice` không cắt cặp surrogate ở 120 / 3000 / 200 / 60 / 80, `resetsAt` vượt Date → Rust bỏ field (TS throw). `.code_index/core.md` cập nhật.
- Dọn: xoá file rỗng `core-transcript/src/{` do 1 lệnh shell hỏng của con tạo ra ở vòng này.
- Test vòng 2: `cargo test --locked -j 4` toàn workspace pass (core-transcript 30 + 2 ignored, core-bin transcript 4/4); clippy `-D warnings` Windows + aarch64-apple-darwin sạch; parity 3 332 file / 51 184 dòng / 3 521 lần đọc / 153 246 signal / 20 loại; `npm test -w @ccm/collector -- --maxWorkers=4` 442 pass / 3 skip / 0 fail; typecheck sạch.
