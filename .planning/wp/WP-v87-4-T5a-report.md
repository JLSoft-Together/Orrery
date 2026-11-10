# WP-v87-4-T5a report — Codex rollout reader in Rust (`CCM_CORE_CODEX`)

## Đã làm
- Rust module `apps/core/crates/core-transcript/src/codex/` (6 file nhỏ + tests): `classify.rs` (`classifyLine` / `TIMESTAMP_RE` viết tay trên 256 byte đầu), `value.rs` (parse JSON chỉ giữ key mà `extractCodexSignals` đọc, object giữ thứ tự JS, key bỏ qua lex bằng `IgnoredAny`, lỗi → retry sau `jsonline::sanitize`), `extract.rs` (`CodexSignal` + `meta_of`, JSON đúng shape TS), `tail.rs` (`RolloutTailer` = `reader.ts`: offset byte, `fromEnd`, dòng dở, skip dòng nội dung, compacted head, truncate → 0, đọc theo trang `maxBytes`, `resume_point`), `meta.rs` (`readSessionMeta` cả nhánh regex 64 KiB), `serve.rs` (op `codex.open|read|meta|close`). `serve.rs` của crate: `codex_op` chạy trước `tracker.*`.
- Ranh giới: chọn bước đệm (a) — Rust đọc + parse, `CodexThreadState` ở lại TS. Lý do (ghi plan §2.4): monitor đọc state đồng bộ ở ~15 chỗ cạnh dữ liệu process / link mà Rust không có; đưa state sang cần mirror như T3 trong khi fold state gần như không tốn CPU.
- Seam TS (file mới trong `apps/collector/src/codex/`): `rolloutSource.ts` (`RolloutSource` / `RolloutSources`, `codexCoreMode`, `createRolloutSources`: `CCM_CORE_CODEX=rust` + `CCM_CORE_BIN` → Rust, mặc định `node`), `nodeRollout.ts` (đường TS cũ), `coreRollout.ts` (`CoreRollouts` / `CoreRolloutSource`: child `orrery-core transcript` riêng, timeout 30 s không restart; reply lỗi → chuyển hẳn sang Node, `reportCoreFallback('codex')` 1 lần, nguồn đang đọc đi tiếp bằng `RolloutTailer` từ `resumeAt` — không mất / không lặp dòng).
- `monitor.ts`: chỉ đổi chỗ dựng reader (`rollouts` = tham số constructor thứ 6, mặc định `createRolloutSources()`), vòng đọc nhận `CodexSignal[][]`, đóng nguồn khi thread bị bỏ. Vẫn 687 dòng (không dài thêm). `reader.ts`: `RolloutTailer` thêm tham số `chunk` + `resumeAt()`. `coreFallback.ts`: kind `codex`.
- Desktop: không đụng. Env `CCM_CORE_CODEX` tự sang collector (`collectorEnv` spread `process.env`).
- Docs: plan §2.4 (protocol + lý do + known differences), `docs/codex-integration.md` 1 dòng, `.code_index/{core,collector,_meta}.md`.

## File đổi
- Mới: `apps/core/crates/core-transcript/src/codex/{mod,classify,value,extract,tail,meta,serve,tests}.rs`, `apps/collector/src/codex/{rolloutSource,nodeRollout,coreRollout}.ts`, `apps/collector/test/codex-parity.test.ts`, `apps/collector/test/codex-parity-corpus.ts`.
- Sửa: `apps/core/crates/core-transcript/src/{lib,serve}.rs`, `apps/collector/src/codex/{monitor,reader}.ts`, `apps/collector/src/coreFallback.ts`, `.planning/PLAN-v87-step4-core-transcript.md`, `docs/codex-integration.md`, `.code_index/{core,collector,_meta}.md`.

## Test
- `cargo test --locked -j 4`: 135 pass / 0 fail (7 test mới trong `codex::tests`).
- `cargo clippy --locked -j 4 -- -D warnings`: sạch (host + `--target aarch64-apple-darwin`).
- `npm test -w @ccm/collector -- --maxWorkers=4`: 476 pass / 0 fail / 3 skip (44 file; `codex-parity.test.ts` 9 test).
- `npm run typecheck -w @ccm/collector`: sạch (gồm cả `test/`).
- Parity (`codex-parity.test.ts`, debug core): 185 lượt đọc TS ↔ Rust so từng lượt, 1837 record, 3 kịch bản `CodexMonitor` (Claude-launched exec + cancel, standalone + subagent con + process thoát + `--resume` mở lại, startup bỏ run đã xong + limits) so snapshot store sau mỗi bước — giống hệt. Fixture lớn sinh trong thư mục tạm (3 seed × ~4 MB): ghi từng mẩu cắt ngẫu nhiên (cả giữa ký tự UTF-8), chunk 4 KiB + trang 7000 byte và mặc định 4 MiB, đọc từ `fromEnd` giữa dòng, truncate rồi ghi lại, ghi tiếp sau truncate; dòng lạ (JSON hỏng, mảng, số, `null`, rỗng, CRLF, key trùng, lone surrogate / `1e400` trong key bỏ qua, `source` lồng 300 tầng, tên tool sai / dài, cwd unicode / 1100 ký tự).
- Fallback: core không trả lời → Node + đúng 1 notification; core chết giữa chừng (sau 4 request, trang 3000 byte) → Node tiếp từ `resumeAt`, tổng record khớp TS.
- Redaction: test kiểm `secret` (base_instructions, arguments, output, message) không có trong output Rust / snapshot; Rust không ghi stderr.

## Số đo CPU-s (script tạm `%TEMP%\ccm-codex-bench\bench.mts`, release core build ở target dir riêng, Windows, file đã nằm trong cache OS, đọc toàn bộ từ đầu, 3 vòng)
| Bộ | TS (`NodeRolloutSource`) | Rust core | Node client (parse reply) |
|---|---|---|---|
| Hỗn hợp `corpus()` 236 MB / 20 file, 7928 record | 0.27–0.38 | 0.20–0.27 | 0.00–0.05 |
| Dòng giữ nặng (tool call / output 7–14 KB) 257 MB / 20 file, 40 000 record | 0.55–0.76 | 0.52–0.55 | 0.06–0.11 |
- Kết luận: lợi CPU nhỏ (~1.1–1.4×). Đường TS vốn đã rẻ vì dòng nội dung (message / reasoning / compacted) không bao giờ được decode, và `JSON.parse` của V8 nhanh; phần lợi chính của T5a là bỏ được việc đọc file khỏi Node (hướng Tauri), không phải CPU.

## Còn lại / ghi chú
- Desktop `main.ts` `noteCoreFallback` chỉ nhận kind `usage` / `transcript` nên fallback `codex` hiện chỉ có log `[ccm] codex rollouts: rust core unusable …` trong collector, chưa hiện Notification / dòng Diagnostics (cần thêm `codex` vào `CORE_READERS` + chuỗi `coreFallbackCodex` trong `strings.ts` — ngoài phạm vi WP này, `apps/desktop/**` cấm đụng).
- Mỗi collector có thêm 1 child `orrery-core transcript` riêng cho Codex khi bật `rust` (tách lỗi khỏi tracker); gộp chung child với tracker để WP sau nếu cần.
- Known differences (plan §2.4): `resets_at` ngoài range Date (TS throw `RangeError`, Rust bỏ `resetsAt`), số ngoài f64 trong key giữ (`used_percent` → 0 % ở Rust), lone surrogate trong chuỗi giữ → U+FFFD, `Date.parse` dạng không ISO.
- Cờ vẫn mặc định `node`; đổi mặc định sang `rust` sau khi T4 / QA trên app chạy thật.
- T5b: hooks (`hooks.ts`), statusline (`statusline.ts`), DeepSeek outbox (`deepseek/`) sang Rust; cân nhắc chuyển `CodexThreadState` + watch `sessions/` sang Rust cùng lúc nếu bỏ Node sidecar.

STATUS: DONE
