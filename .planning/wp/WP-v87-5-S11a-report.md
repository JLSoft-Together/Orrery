# WP-v87-5-S11a — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S11a.md`. Không commit / push / checkout, không sửa `apps/collector/src`, `packages/shared`, `apps/web`, `apps/desktop/src`. Mọi lần chạy `serve` / collector: `CCM_PORT` trống (≠ 4317), data / Claude / Codex / Codewhale home tạm; không chạm app thật; chỉ kill pid tự spawn. CPU trước mỗi việc nặng 28–35 %, mỗi lúc 1 việc nặng.

## Đã làm

1. **Đọc file ngoài store lock** — `Monitor::presync(now)` (file mới `core-monitor/src/monitor/presync.rs`), `serve` gọi ngay sau `prepare`, ngoài `handle.with`:
   - drain watcher (registry / project events), rồi chạy trước mọi `MonTracker::sync` mà `tick` sẽ gọi, đúng thứ tự của `tick`: reconcile do registry → file project đến hạn (`Files`) → reconcile 5 s. Reconcile dùng `pre_registry` của `prepare`: terminal mất khỏi registry → `All` (như `end_tracker`), cùng session + cwd → `reconcile_scope` (như cũ); session vừa đổi id / cwd → để lại đọc dưới lock (đúng ngữ nghĩa `set_record` reset epoch).
   - Kết quả mỗi lần sync xếp hàng `Staged { drafts, spans, changed }` theo terminal. Dưới lock `sync_tracker` lấy kết quả đã đọc (`take_synced`: `Staged` / `Covered` = đã reconcile, không đọc lại / `Read` = đọc như cũ). `publish`, push activity, `tool_span`, peers vẫn dưới lock (rẻ, ~0,1 ms).
   - `tick` dùng chung mốc giờ với presync (`tick_wall`) và không drain watcher lần 2 (event đến sau nằm chờ presync kế, không bị mất). `reconcile` clone `pre_registry` (2 reconcile trong 1 tick không đọc registry dưới lock), tracker lấy từ `pre_trackers` không catch-up lần 2 dưới lock (`pre_used`). `end_tick` dọn hết sau `tick` / `start`. Không gọi `presync` (bin `monitor-script`, twin, test cũ) = hành vi cũ.
   - Còn đọc dưới lock (hiếm): `verify_live` kết thúc terminal (60 s, chỉ khi bật verify), session đổi id / cwd.
   - `stats.rs`: thêm `outsideLockMs` (prepare + presync). Script đo mới `scripts/trace-lockstats.mjs` (demo 30×6 tick 400 ms 60 s + `orrery-core serve --dev` `CCM_SERVE_STATS=1`, in `serve.stats` + `/health`).
2. **Lệch trace lượt b của S10** — nguyên nhân: **lệch thời điểm đọc, không phải lệch logic** (chi tiết dưới).

## File

Mới: `apps/core/crates/core-monitor/src/monitor/presync.rs`, `apps/core/crates/core-monitor/tests/presync.rs`, `apps/collector/test/subagent-race-twin.test.ts`, `scripts/trace-lockstats.mjs`, báo cáo này.
Sửa: `apps/core/crates/core-monitor/src/monitor/{mod,reconcile}.rs`, `apps/core/crates/core-serve/src/{runtime,stats}.rs`, `.code_index/core.md`, `.planning/PLAN-v87-step5-core-store.md` (§3.9 1 câu + §3.10 mới).
Chưa ghi `.code_index/_meta.md` (ngoài phạm vi brief) — main thêm 1 dòng nếu muốn.

## Test (số thật)

- `cargo clippy --locked -j 4 --workspace --all-targets -- -D warnings`: 0 cảnh báo.
- `cargo test --locked -j 4 --workspace`: 456 pass, 0 fail, 6 ignored. Mới `tests/presync.rs` 2: (a) presync rồi ghi thêm dòng vào transcript trước `tick` → store y hệt `tick` thường và dòng mới không có (tick không đọc file), presync kế mới thấy; (b) session mất khỏi registry: đọc `All` ở presync, dưới lock chỉ end (`terminal.ended`, dòng ghi sau presync không bị đọc). `tests/prepare.rs` vẫn pass.
- `npm test -w @ccm/collector -- --maxWorkers=4`: 52 file pass / 1 skip, 537 pass / 3 skip (twin monitor / agents / toast / sources / persist + race twin mới 2). `npm run typecheck -w @ccm/collector`: sạch.

## Số đo store lock (`node scripts/trace-lockstats.mjs`, demo 30×6, tick 400 ms, 60 s, release)

| | lock trong tick p50 | p99 | max | `/health` p99 |
|---|---|---|---|---|
| Trước (HEAD, `orrery-core` S10) | 0,32 ms | 514 ms | 1 080 ms | 7,5 ms |
| Sau | 0,33 ms | **2,4 ms** | **4,0 ms** | 3,5 ms |

Đạt cổng p99 ≤ 20 ms, max ≤ 100 ms. Phần đọc dời ra ngoài lock (`outsideLockMs`) p99 369 / max 1 004 ms: vòng monitor vẫn tuần tự, nên event tới WS vẫn đi sau lượt đọc đó; WS snapshot / flush / `/health` không còn bị chặn (frame trong 60 s: 130 → 250). Đo trong lúc thử: thời gian là `Tracker::sync` đọc file (mở + đọc phần thêm ~5–7 ms / file trên Windows, 3–20 subagent mỗi session), `publish` ~0,1 ms. Giảm chi phí đọc (giữ handle, gom lượt) là việc sau nếu cần độ trễ event bằng Node.

## Trace `--server=both` (bản cuối, demo 30×6, mỗi lượt ~175 s)

| Lượt | Frame Node / Rust | Khác biệt (fold) |
|---|---|---|
| 1 | 866 / 792 | 0 |
| 2 | 858 / 777 | 0 |
| 3 | 858 / 769 | 0 |
| 4 | 850 / 782 | 0 |
| 5 | 858 / 767 | 0 |

## Nguyên nhân lệch lượt b (S10)

Demo `finishSubagent` ghi `end_turn` vào file subagent (T), rồi `task-notification` (status 20 % `failed` / `stopped`) vào main (T + vài ms; harness `--split` 250 ms giữa 2 file). Quy tắc resolve của cả Node lẫn Rust (`core-transcript/src/tracker/subs.rs` `resolve` / `linked_outcome`): có notification / foreground result → lấy trạng thái đó và giờ của nó; chưa có mà subagent đã `ended` + không tool mở → `completed` tại `last_end_at`; đã resolve thì không đổi. Nên:

- đọc subagent **trước** khi main có notification → `completed` tại T;
- đọc cả 2 trong **một** lượt (main đọc trước subagent) → `error` tại T + 2 ms (`agent.failed`).

Đúng hình lượt b: Node `completed`, Rust `error`, `completedAt` +2 ms. Bản S10 giữ lock / chạy tick tới ~1 s nên Rust gộp 2 lượt ghi cách 250 ms vào một lượt đọc. Chứng minh bằng test tái hiện `apps/collector/test/subagent-race-twin.test.ts`: Node `Monitor` + `monitor-script` trên cùng fixture, 2 thứ tự (`sub-then-main` / `together`) → mỗi thứ tự Node = Rust (strict + fold 0), cả hai cho `completed`@T / `error`@T+2 ms theo thứ tự đọc. Không có lệch logic, không sửa. 5 lượt bản cuối không tái hiện.

## Còn lại

- Độ trễ event vẫn theo lượt đọc file tuần tự (`outsideLockMs` p99 ~370 ms ở stress 30×6) — race trên vẫn có thể xảy ra khi máy bận (cả Node cũng vậy); chỉ là thời điểm.
- `verify_live` end + session đổi id vẫn đọc dưới lock (hiếm).
- Việc S11 khác (E2E Win + mac `CCM_CORE=rust`, RAM / CPU, đổi mặc định) chưa thuộc gói này.

STATUS: DONE
