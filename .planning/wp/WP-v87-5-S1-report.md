# WP-v87-5-S1 — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S1.md`. Không commit / push, không sửa `apps/collector/src`, `apps/web`, `apps/desktop`, `packages/shared`. Không đụng app Orrery đang chạy (collector thu trace luôn chạy cổng trống + thư mục tạm của `trace-capture`).

## Đã làm

1. **`Store` Rust** (`apps/core/crates/core-store/src/store/`): bản 1:1 của `apps/collector/src/store.ts`.
   - `mod.rs` (282 dòng): maps terminal / agent / link (`IndexMap`, giữ thứ tự chèn như `Map`, xoá bằng `shift_remove`), `agent_ids_by_terminal` (`IndexSet`), `upsert_*` / `remove_*` (xoá terminal xoá agent trước), `push_activity` (2 ring `VecDeque` theo `activity_limit`, ring riêng cho `tool.*`, id = base36(now)-base36(counter), `at` mặc định = giờ clock), `agents_of`, `context_pct` (`contextUsedPct` buffer 16,5 %), `current_tool_of`, `tool_events`, `peer_links`.
   - `settings.rs` (232): mọi setter — `set_loading`, `set_notify`, `set_snoozes`, `reset_usage` / `update_usage` / `set_usage_scan` / `set_project_roots` (+ `set_on_new_projects`, gọi sau emit như Node), `set_favorites`, `reject_favorite`, `history_data` / `history_added`, `tool_span`, `timeline_data`, `folder_picked`, `focus_result`, `set_jobs`, `set_limits` (bỏ `None`), `set_codex_limits`, `set_response`, `set_model_names`, `set_sensitive`, `set_resources` (bằng nhau khi bỏ `at`); so bằng chuỗi JSON như `JSON.stringify`.
   - `batch.rs` (167): `emit` + coalesce theo `(EntityKind, id)` (updated gộp vào created / updated cùng batch, field cuối thắng, `null` xoá field trong created, `removed` cắt chuỗi), `flush` (tăng `seq`, listener lỗi chỉ log, listener khác vẫn nhận), `subscribe` / `unsubscribe`, `next_flush_at` / `poll(now)` (batch 50 ms do chủ chạy — S2 gắn tokio), `snapshot` (activity sort theo `at`, key có / vắng như Node), `snapshot_message` (flush trước) / `snapshot_now`.
   - Diff bắt chước `===` (quyết định main): `entity!` sinh `diff` / `apply` / `merge`; `jseq.rs` `JsEq` — scalar so giá trị (`JsNum`: `0 === -0`, `NaN !== NaN`), field object (`TerminalSession.denied_tools` / `diff` / `limit_hit`, đổi sang `Arc`, serde feature `rc`) so `Arc::ptr_eq` → object mới cùng nội dung vẫn vào patch như Node. `time.rs`: `iso_of_ms`, `base36`, so chuỗi UTF-16. Clock tiêm được (`Store::with_clock`).
2. **Seam phát lại** — ghi chuỗi setter từ Node, phát lại vào Store Rust, xuất trace `ccm-trace/1`:
   - `scripts/trace-store-record.mjs` `instrumentStore`: bọc method `MonitorStore` (chỉ lệnh cấp ngoài cùng; flush ghi trước khi listener chạy để lệnh listener gọi lại store đứng sau; `dropped` khi flush không ra frame nào; id đối tượng qua `WeakMap` → `refs`; `now` / `at` của activity). Không sửa `store.ts`.
   - `scripts/trace-store-hook.mjs` (`--import`, env `CCM_TRACE_OPS`): bọc `MonitorStore.prototype` + `ws` `send` (bắt direct reply, đếm frame) trong process collector, ghi NDJSON mỗi 200 ms.
   - `scripts/trace-capture.mjs --ops=<file>`: thêm hook, ghi giờ mỗi step, gắn mốc step vào chuỗi op (`withSteps`) → `ccm-store-ops/1`.
   - Rust `src/replay.rs` + bin `store-replay <ops.json> <out.trace.json>`: lỗi rõ khi gặp lệnh lạ hoặc field object chưa có luật identity.
   - `scripts/trace-replay.mjs <ops> <node.trace> [--out=] [--bin=]`: chạy binary (`CCM_STORE_REPLAY` hoặc `apps/core/target/{debug,release}`) rồi so `--strict` + fold.
   - `apps/collector/test/trace-scenario.ts`: `monitorScenario({ ops })` ghi op khi được yêu cầu (mặc định như cũ).
3. **Test chuyển sang Rust** (`tests/store.rs`, 15 test) + test replay (`tests/replay.rs`, 3) + unit `jseq` / `time` (4).
4. Plan §3.2 (cách chạy, kết quả, giới hạn, ý tưởng so sâu để sau), `.code_index/{core,collector,_meta}.md`.

### Đối chiếu test Node → Rust

| Node | Rust |
|---|---|
| `units.test` MonitorStore "emits created, field patches with null for cleared fields, and batches" | `store.rs` `emits_created_field_patches_with_null_and_batches` (batch qua `next_flush_at` / `poll`) |
| `units.test` "merges patches per id in a batch, last value per field, without crossing a removal" | `merges_patches_per_id_without_crossing_a_removal` |
| `units.test` "keeps broadcasting when one listener throws" | `keeps_broadcasting_when_one_listener_fails` (listener trả `Err`) |
| `units.test` "emits only new roots and reports unknown projects once resolved" | `emits_only_new_roots_and_reports_unknown_projects` |
| `store-diff.test` "emits nothing for an identical upsert, even with undefined fields" | `identical_upsert_emits_nothing_even_with_absent_fields` |
| `store-diff.test` "patches only changed fields and nulls fields that became undefined" | `patches_only_changed_fields_and_nulls_cleared_ones` |
| `store-diff.test` "agentsOf follows upsert, terminal move and removal" | `agents_of_follows_upsert_move_and_removal` |
| `monitor.test` phần store (kịch bản `trace-scenario.ts`) | `replay.rs` `monitor_scenario_replays_to_the_node_frames` (frame bằng hệt, kể cả `seq`, id activity) + `trace-replay.test.ts` |
| (mới) | `object_fields_patch_when_identity_changes_like_triple_equals`, `created_absorbs_later_patches_and_null_drops_the_field`, `remove_terminal_removes_its_agents_first`, `links_create_then_patch`, `setters_skip_json_equal_values`, `snapshot_sorts_activity_by_time_and_keeps_rings_apart`, `snapshot_message_flushes_first_and_carries_the_seq`, `context_pct_and_current_tool_come_from_the_main_agent`, `replay_drops_flushes_nobody_received_and_keeps_object_identity`, `replay_reports_unknown_calls_and_unruled_object_fields` |

`hardening.test`, `toast.test`, `job-actions.test`… dùng `MonitorStore` làm phụ kiện cho server / toast / monitor, không test store — chuyển ở S2 / S7 / S4.

## File

- Mới: `apps/core/crates/core-store/src/{jseq,time,replay}.rs`, `src/store/{mod,settings,batch}.rs`, `src/bin/store-replay.rs`, `tests/{store,replay}.rs`, `tests/replay/monitor.{ops,node.trace}.json` (33 KB + 23 KB, đường tạm → `<root>`), `scripts/trace-store-record.mjs` (120 dòng), `scripts/trace-store-hook.mjs` (34), `scripts/trace-replay.mjs` (78), `apps/collector/test/trace-replay.test.ts` (95).
- Sửa: `apps/core/crates/core-store/{Cargo.toml (serde rc), src/lib.rs, src/macros.rs, src/model.rs (3 field Arc)}`, `apps/collector/test/trace-scenario.ts`, `scripts/trace-capture.mjs`, `.planning/PLAN-v87-step5-core-store.md`, `.code_index/{core,collector,_meta}.md`.
- `Cargo.lock` không đổi. `.planning/PROGRESS-2026-11-09-v87.md` đang modified nhưng không phải do WP này. Mọi file ≤ 360 dòng.

## Test (số thật)

- `cargo test --locked -j 4` (apps/core, PowerShell): 175 passed, 5 ignored, 0 failed (S0: 153); `core-store` 33 = unit 6 + golden 4 + replay 3 + shared_ts 5 + store 15.
- `cargo clippy --locked -j 4 --all-targets -- -D warnings`: sạch.
- `npm test -w @ccm/collector -- --maxWorkers=4`: 46 file passed + 1 skipped, 500 passed, 3 skipped (S0: 497; +3 ở `trace-replay.test.ts`, test replay chạy thật vì đã có binary).
- `npm run typecheck -w @ccm/collector`: sạch.

## Trace Node vs Rust

Mỗi lượt: một Node collector thật (`trace-capture --ops`) → trace WS Node + chuỗi op; phát lại op vào Store Rust → trace Rust; so cùng lượt.

| Lượt | Op | Frame Node / Rust | `--strict` | fold | Ghi chú |
|---|---|---|---|---|---|
| demo 30×6 a (CPU 19 % lúc đầu) | 54 932 | 828 / 828 | 0 | 0 | so thô từng frame (canonical, kể cả `seq`, id activity): 0 khác |
| demo 30×6 b (CPU 33 %) | 58 199 | 840 / 840 | 0 | 0 | dropped 3 frame (flush trước khi có client, như Node), refs dùng lại 0, mutated 0, clock skew 0 |
| `monitor` trong process (vitest + cargo fixture) | 94 | 20 / 20 | 0 | 0 | frame bằng hệt |

Lưu ý thật: so **Node a ↔ Node b** (2 lần chạy collector Node) lượt này **không** bằng 0 (50+ khác biệt, bắt đầu ở step 12 `end`: agent `status` `error` ↔ `completed`, `toolUseId` thiếu; 828 ↔ 840 frame). Đó là lệch thời điểm đọc file của chính Node collector (giới hạn đã ghi ở plan §3.1, lượt b chạy 196 s so với 176 s), không liên quan Store — mỗi lượt Rust replay vẫn khớp đúng Node của lượt đó. Store được so trên chuỗi lệnh Node đã gọi, nên kết quả không phụ thuộc lệch này.

## Việc cho S2 / S3 / S4

- **S2**: chạy batch bằng tokio quanh `Store` (`next_flush_at` / `poll`; `Store` là `Send`, bọc `Mutex` hoặc actor); khi client nối: `flush()` rồi `snapshot_message()`; WS gửi `ServerMessage` serialize sẵn 1 lần cho mọi client. Listener trong Rust không được gọi lại store (borrow) — toast / snooze prune (`index.ts` `observe` gọi `setSnoozes` trong listener) phải xếp lệnh sau flush; replay đã cho thấy thứ tự "flush rồi lệnh listener" cho cùng output.
- **S3**: `set_snoozes` / `set_notify` / `set_favorites` nhận dữ liệu từ store nhỏ; giữ thứ tự key `Dict` (`IndexMap`) như file JSON để so `JSON.stringify` khớp.
- **S4**: monitor Rust phải giữ `Arc` cũ cho `diff` / `deniedTools` / `limitHit` khi Node giữ tham chiếu cũ (và tạo `Arc` mới khi Node tạo object mới) để `--strict` khớp; replay hiện không có dữ liệu thật cho nhánh này (demo không có git stat / denied / limit hit) — cần fixture có repo git + denied tool + limit hit. `trace-capture --server=rust` khi có `orrery-core serve`; dùng `trace-replay` để tách lỗi store khỏi lỗi monitor.

STATUS: DONE
