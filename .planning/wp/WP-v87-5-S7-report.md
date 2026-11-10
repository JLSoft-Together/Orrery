# WP-v87-5-S7 — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S7.md`, dòng S7 plan `PLAN-v87-step5-core-store.md` (§3.7 mới). Không commit / push, không sửa `apps/collector/src`, `apps/web`, `apps/desktop`, `packages/shared`, không đụng file của S5 / S8. Không chạm app Orrery đang chạy (không mở cổng, không đọc / ghi `%APPDATA%\Agent Monitor` / `%LOCALAPPDATA%\ccm`; `notify.json` trong test nằm ở thư mục tạm). Không gửi toast / phone / Telegram thật: mọi output chỉ là message trong memory. CPU lúc chạy test 6–66 % (một lần đo 96 % khi S5 / S8 đang build: chờ rồi mới chạy cả bộ).

## Đã làm

1. **Crate mới `apps/core/crates/core-toast`** (lib + bin `toast-script`), port 1:1 phần logic của `apps/collector/src/toast.ts` (phần vẽ `ps-host` PowerShell không port; main hiển thị):
   - `toaster/mod.rs` `Toaster`: prefs + link key qua `core_store::persist::notify_file::NotifyFile` (S3, cùng file `notify.json`), `settings()` → `NotifySettings`, `update(&NotifyPatch)` (+ `NotifyPatch::from_client` cho message `notify.update` của core-http), presence / locale, `link_for`, `snooze_link`, `accepts` (sha256 + so constant-time), `test`, `hide_for`, `clear`, `drain()`.
   - `toaster/observe.rs` `observe(&ToastSnapshot)`: waiting (throttle 30 s, nút snooze), reminder (max 3 lần, mỗi 5 phút), done, Telegram (`question` / `permission` / `waiting` / `done` + `reply`, `ask`), phone (waiting / question / permission, done khi làm ≥ 5 phút, job, limit forecast, limit hit 1 lần mỗi window + reset), stuck, context ≥ 95 %, job blocked, forecast 5h.
   - `toaster/extra.rs`: `error_loops` (= `errorLoops` shared), `check_loops`, `observe_sensitive`, `observe_disk`, `observe_note`, `observe_idle`.
   - `message.rs`: `ToastContent`, `PhoneEpisode`, `TelegramEpisode`, `HostMessage::to_message(full)`. Shape JSON đúng như Node gửi qua `parentPort`: `{type:'phone',…}`, `{type:'telegram',…}`, `{type:'toast.show', tag, title, persistent}` / `{type:'toast.hide', tag}` (`full = false` = đúng bản tối thiểu `electronToastDeps`); `full = true` thêm `body` / `url` / `open` / `dismiss` / `snoozeUrl` / `snoozeLabel` cho main vẽ toast Windows.
   - `text.rs` (chuỗi en / vi, `rule_label`), `util.rs` (helper giữ hành vi JS: `toFixed(1)` làm tròn nửa lên, `String(number)`, `encodeURIComponent`, `path.win32.basename`, `waitReason`, `inQuietHours`, `remindersDue`, `quietFor`, `formatGb`, `formatBytes`, `serverLabel`, `toLocaleTimeString` HH:MM).
   - Telegram `reply_text` ≤ 3000: Rust dùng `Monitor::reply_of` (S4, tracker đã cắt 3000, chỉ trong memory) qua closure `reply_of` của `ToastSnapshot`; không ghi đĩa, không ra trang web.
2. **Harness** `script.rs` + bin `toast-script` (NDJSON, đồng hồ giả).
3. **Test chuyển 1:1** `toast.test.ts` → `tests/toast.rs` (24) + `tests/episodes.rs` (7): 18 case `Toaster`, `SnoozeStore` (dùng `core_store` SnoozeStore), v11 ×4, link key qua restart, disk, notes, limit ×3; thêm Telegram + phone episode và idle report. 3 case route (`app key`, `focus routes`, `snooze route`) đã có ở `core-http/tests/hardening.rs` (S2). `toast-electron.test` → unit `message.rs` (`to_message(false)` chỉ gửi tối thiểu).
4. **Twin** `apps/collector/test/toast-twin.ts` + `toast-twin.test.ts`: Node `Toaster` (deps ghi lại, `onPhone` / `onTelegram`) và `toast-script` chạy cùng chuỗi lệnh.
5. Docs: plan §3.7 + đánh dấu dòng S7, `.code_index/{core,collector,desktop,_meta}.md`.

## File (khớp `git status`)

- Mới: `apps/core/crates/core-toast/` (`Cargo.toml`; `src/{lib,message,script,text,util}.rs`, `src/toaster/{mod,observe,extra}.rs`, `src/bin/toast-script.rs`, `tests/{toast,episodes}.rs`, `tests/common/mod.rs`), `apps/collector/test/toast-twin.ts`, `apps/collector/test/toast-twin.test.ts`, báo cáo này.
- Sửa: `apps/core/Cargo.toml` (thêm `"crates/core-toast"`), `apps/core/Cargo.lock` (+13 dòng: chỉ gói `core-toast`; `getrandom` 0.4.3, `sha1` 0.11, `sha2` 0.11 đã có sẵn trong lock), `.planning/PLAN-v87-step5-core-store.md` (dòng S7 + §3.7), `.code_index/core.md`, `collector.md`, `desktop.md` (1 câu ở mục `attention.ts`), `_meta.md` (append).
- Không phải của S7: `core-monitor/**` (S4 / S5 / S8), `apps/collector/test/monitor-twin.ts`, `agents-twin.test.ts`, `apps/desktop/**`, `apps/collector/.vitest/` (tạo lúc 13:13 bởi một lần chạy vitest có reporter json, không phải lệnh của S7), `.planning/PROGRESS-*`, `RELEASE-*`, `wp/WP-v87-5-S5*`, `S8*`, `docs/qa/*`.
- Mọi file ≤ 353 dòng, không comment.

## Test (số thật)

- `cargo test --locked -j 4` (apps/core, cả workspace, gồm code S5 / S8 đang làm): **425 passed, 0 failed, 6 ignored**. `core-toast`: unit 6 + `toast.rs` 24 + `episodes.rs` 7 = **37**.
- `cargo clippy --locked -j 4 -p core-toast -p core-store --all-targets -- -D warnings`: sạch. Clippy cả workspace hiện **lỗi ở `core-monitor/src/deepseek/{monitor,process}.rs`** (`manual_range_contains`, file của S5 đang làm), không phải của S7.
- `npm test -w @ccm/collector -- --maxWorkers=4`: **51 file passed + 1 skipped, 535 passed, 3 skipped** (gồm 7 test twin mới; `toast.test` / `toast-electron.test` Node giữ nguyên và pass).
- `npm run typecheck -w @ccm/collector`: exit 0. Desktop không đụng nên không chạy test desktop.

## Kết quả parity

| Chuỗi | Bước | Toast op | Phone / Telegram | Khác biệt |
|---|---|---|---|---|
| scripted | 37 | 24 | 12 | 0 |
| fuzz seed 1 | 400 | 135 | 109 | 0 |
| fuzz seed 2 | 400 | 96 | 117 | 0 |
| fuzz seed 3 | 400 | 149 | 113 | 0 |
| fuzz seed 4 | 400 | 67 | 140 | 0 |
| fuzz seed 5 | 400 | 84 | 145 | 0 |
| fuzz seed 6 | 400 | 89 | 82 | 0 |

So mỗi bước: kết quả lệnh (settings, state, test, accepts, link, snoozeLink, key lúc init), phone / Telegram đúng thứ tự và deep-equal, toast show (đủ field) / hide theo từng tag đúng thứ tự. Fuzz phủ: waiting / question / permission / idle / working, statusSince, lastActivityAt, limitHit 3 window, endedAt, xoá terminal, contextPct (94.5 / 95 / 95.5 / 99), toolOf / replyOf / askOf, jobs, limits (cả `null`), snooze, presence + locale, update kinds / quiet / stuck / idle / toast (cả giá trị sai), loops, sensitive, disk, note, idle report, hideFor / clear / test. Thử đột biến: đổi throttle Rust 30 s → 90 s thì twin lệch ở 3 seed (đã trả lại, file giống hệt bản trước).

## Khác biệt biên (cố ý / đã biết)

- `test()` trả `ok` ngay vì main vẽ toast bất đồng bộ; kết quả thật + `health` Windows main phải báo lại (`set_health`, hoặc `health_probe`) — S10.
- Thứ tự show / hide giữa 2 tag khác nhau trong một lượt có thể khác Node (Node xếp hàng Promise theo tag); trong một tag đúng thứ tự. Twin so theo tag.
- `Date.parse` chỉ ISO; `slice(0, n)` không cắt đôi cặp surrogate; `contextPct` NaN coi như thấp (Node coi là cao); `formatBytes(NaN)` ra `1 MB` (Node `NaN MB`).
- Message `toast.show` cho macOS (`attention.ts`) dùng `to_message(false)`, Windows main cần `to_message(true)`.

## Việc cho WP sau

- S10: tạo `Toaster` trong `serve` (`ToastOptions { supported: CCM_TOAST != 0 (và backend), port, file: data/notify.json, link: CCM_TOAST_LINK }`, `load()`, `phone_on` / `telegram_on` khi có main), thay `notify_settings()` tạm của `monitor-serve` bằng `toaster.settings()`; gọi `observe` sau mỗi `tick` (như listener flush Node) + timer 15 s cùng `check_loops(store.tool_events())`; `observe_sensitive` cho activity `sensitive`; `observe_idle` ở sampler (S6) và `idle_watch()` cho `setActive`; client message `client.presence` / `notify.update` / `notify.test` / `terminal.snooze` (+ `hide_for`), route `/focus` / `/snooze` dùng `accepts` + `hide_for`; stdin `diskAlert` / `agentNote` → `observe_disk` / `observe_note`; `drain()` → stdout NDJSON; main (`main.ts` nhánh `phone` / `telegram`, `attention.ts`) nhận như hiện tại, Windows thêm vẽ toast từ `full` message; `poll` / `flush` `notify.json`.
- `snoozed` là closure mượn tạm (`rt.snoozes.is_snoozed`), không cần `Arc<Mutex>`.

STATUS: DONE
