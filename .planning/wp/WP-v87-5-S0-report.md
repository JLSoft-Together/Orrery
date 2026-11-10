# WP-v87-5-S0 — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S0.md`. Không commit / push, không đụng `apps/collector/src`, `apps/web`, `apps/desktop`, `packages/shared`.

## Đã làm

1. **Thu golden trace WS từ Node collector** — `scripts/trace-capture.mjs`: demo-fixture 30×6 seed 8704 phát lại 10 bước vào Claude root tạm, collector thật `--dev` (`node --import tsx`) trên cổng trống (từ chối 4317), `CCM_DATA_DIR` / `CODEX_HOME` / `CODEWHALE_HOME` tạm, mọi `CCM_CORE_*=node`, `CCM_RES_FIXTURE` rỗng; ghi snapshot + mọi `ServerMessage` + direct reply theo 16 bước (boot, resync, replay 2–10, client msg, end, queries, dismiss, final). Pid trong registry là process giữ chỗ do script spawn / kill. Thư mục tạm xoá khi xong (`--keep` để giữ).
2. **So trace** — `scripts/trace-compare.mjs` (CLI + export `digest` / `compareTraces` / `makeNormalizer`). Quy tắc chuẩn hoá ghi ở plan §3.1: bỏ `seq` / batch / thứ tự key / activity id; thời gian thật (ISO, epoch ms, ngày hôm nay, `durations`) → placeholder, thời gian logic giữ; alias pid + xoay peer link sau alias; đường tạm; fold state như client + multiset fired / direct + snapshot resync; `--strict` so cả chuỗi event phẳng; `ignore` mặc định `resources`.
   - **Node vs Node = 0 khác biệt, 2 lượt**: a↔b 0, c↔d 0 (thêm a↔d 0). Mỗi trace 816–865 frame, ~23 100 event, ~175–193 s.
   - Phát hiện: Node collector tự nó lệch giữa 2 lần chạy khi file được ghi dồn (watch Windows rơi event; task-notification đọc trước dòng cuối subagent → span `interrupted` thay vì `ok`, agent thiếu token, usage thiếu message). Harness xử lý bằng thứ tự ghi subagent → 250 ms → main, và lượt `utimes` chạm lại file sau 500 ms. Không sửa collector.
3. **Crate `apps/core/crates/core-store`** (khung, chưa có state): type Rust viết tay khớp `packages/shared` — `TerminalSession` / `Agent` / `PeerLink` + patch (`Option<Option<T>>`: vắng / `null` xoá / giá trị), `ActivityEvent`, usage, limits, jobs, notify, snooze, spans, history, timeline, heatmap, toolstats, diagnostics, sensitive, `MonitorSnapshot`, `MonitorEvent` (43 loại, tag `type` / `payload`), `ServerMessage`. `JsNum` in số nguyên như `JSON.stringify`. Payload để `serde_json::Value` tạm: `resources` (S6), `office.snapshot` (S9), `promptSearch.progress|done` (S6).
   - **Không dùng `ts-rs`**: nó sinh TS từ Rust (ngược chiều — `packages/shared` là nguồn), thêm crate mới cần mạng, và `Patch<T>`, union chuỗi, `Record` map tay đơn giản hơn. Thay bằng test đọc thẳng `types.ts`: mọi field interface, mọi union chuỗi, 43 loại event, hằng `PROTOCOL_VERSION` / `SUPPORTED_CLAUDE_VERSION` phải khớp Rust (`tests/shared_ts.rs`) — TS đổi mà Rust chưa đổi thì test đỏ.
   - Round-trip golden: `tests/golden/demo-30x6.json` (44 message, 26 loại event, 195 KB) + `monitor-fixture.json` (29 message, 15 loại) do `scripts/trace-samples.mjs` lấy từ trace golden (mỗi loại tối đa 4 message nhỏ nhất có đường field mới, ≤ 100 KB; pid → 10000+i, đường tạm → `<root>` / `<data>` / `<tmp>`, không còn đường máy). Parse → serialize → so từng field (số so theo giá trị) = khớp hết; `deny_unknown_fields` bắt field thiếu.
4. **Kịch bản trong process** — `apps/collector/test/trace-scenario.ts` + `trace-golden.test.ts`: kịch bản kiểu `monitor.test` (tool result, `toolSpans.get`, alias / note, subagent lồng + fail, subagent xong, rename / waiting / shell / `/clear` / end / TTL) chạy 2 lần → strict = 0 và fold = 0; `CCM_TRACE_OUT=<dir>` ghi trace; 4 test đơn vị cho bộ chuẩn hoá.
5. Plan §3.1 (cách chạy harness, quy tắc chuẩn hoá, kết quả, giới hạn), `.code_index/{core,collector,_meta}.md`.

## File

- Mới: `scripts/trace-capture.mjs` (340 dòng), `scripts/trace-compare.mjs` (316), `scripts/trace-samples.mjs` (91), `apps/collector/test/trace-scenario.ts` (206), `apps/collector/test/trace-golden.test.ts` (133), `apps/core/crates/core-store/{Cargo.toml, src/{lib,macros,num,model,extra,diagnostics,event}.rs, tests/{shared_ts,golden}.rs, tests/golden/{demo-30x6,monitor-fixture}.json}`.
- Sửa: `apps/core/Cargo.toml` (member), `apps/core/Cargo.lock` (+`core-store`, `indexmap` thêm feature `serde`; `cargo update --workspace --offline`, không đổi version nào), `.planning/PLAN-v87-step5-core-store.md` (§3.1), `.code_index/core.md`, `.code_index/collector.md`, `.code_index/_meta.md`.
- `.planning/PROGRESS-2026-11-09-v87.md` đang modified trong git status nhưng không phải do WP này.

## Cách chạy harness

```
node scripts/trace-capture.mjs --out=%TEMP%\ccm-s0\a.trace.json
node scripts/trace-capture.mjs --out=%TEMP%\ccm-s0\b.trace.json
node scripts/trace-compare.mjs %TEMP%\ccm-s0\a.trace.json %TEMP%\ccm-s0\b.trace.json
node scripts/trace-samples.mjs <trace...> --out=apps/core/crates/core-store/tests/golden/<name>.json
npm test -w @ccm/collector -- test/trace-golden.test.ts
cd apps/core; cargo test --locked -j 4 -p core-store
```

Chỉ chạy khi máy rảnh (CPU > 80 % làm Node lệch thời điểm đọc). Chi tiết tham số: plan §3.1.

## Test (số thật)

- `cargo test --locked -j 4` (apps/core, PowerShell): 153 passed, 5 ignored, 0 failed; trong đó `core-store` 11 (2 unit + 4 golden + 5 shared_ts).
- `cargo clippy --locked -j 4 --all-targets -- -D warnings`: sạch.
- `npm test -w @ccm/collector -- --maxWorkers=4`: 45 file passed + 1 skipped, 497 tests passed, 3 skipped.
- `npm run typecheck -w @ccm/collector`: sạch.
- Trace Node vs Node: 2 lượt, 0 khác biệt.

## Việc cho S1

- Store Rust phải giữ ngữ nghĩa Node: `diff` so `===` nên field object / mảng (`diff`, `limitHit`, `deniedTools`) vào patch mỗi khi monitor tạo object mới dù nội dung giống — fold không thấy khác, `--strict` thì thấy; quyết định bắt chước hay so sâu (ghi vào plan).
- Coalesce: update gộp vào created / updated cùng batch, `null` xoá field trong created, `removed` cắt chuỗi; `setX` bỏ qua khi `JSON.stringify` bằng nhau; `setResources` so bỏ `at`; snapshot sort activity theo `at`; peer link id `a<b` theo chuỗi.
- Thêm `--server=rust` vào `trace-capture.mjs` khi có `orrery-core serve` (S2/S10) để so Rust ↔ golden Node; fixture thật 27 MB + 41 sub chưa có kịch bản (S4).
- Thay `Value` của `resources` / `promptSearch.*` (S6), `office.snapshot` (S9) bằng type.

STATUS: DONE
