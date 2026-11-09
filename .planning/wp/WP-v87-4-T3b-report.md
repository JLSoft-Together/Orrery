## WP-v87-4-T3b — DONE

- Đã đổi:
  - Rust `core-transcript`: `prune.rs` mới (152 dòng, `Pruned(Shape)` serde seed: chỉ dựng `Value` cho các key `extract_signals` đọc; phần còn lại lex bằng `IgnoredAny`, không cấp phát: tool result to, thinking, `toolUseResult.file`…), `jsonline::attempt` dùng nó (cho cả `tracker.*` lẫn `signals.*`). `tail.rs` `read_some` mở file 1 lần, lấy size từ handle (trước: `metadata` + `open`). `tracker/view.rs` + `mod.rs`: view delta theo field (`sent_main` / `sent_subs`; chỉ key đổi, key mất = `null`, `state` của sub lồng cùng cách, sub mới gửi nguyên; không gì đổi → không có `view`).
  - Collector: `trackerView.ts` `mergeView` patch theo field (giữ identity object không đổi → `store.diff` thấy `===`). `coreTracker.ts` `takeChanged()` (view merge / record đổi (so nông) / end đổi) + option `safetyNetMs` / `clock`. `trackerSource.ts` `SAFETY_NET_MS` 4 s truyền cho `CoreTracker`, interface `takeChanged?`. `monitor.ts` sửa đúng 1 dòng trong `syncTracker`: `if (tracker.takeChanged?.() !== false) this.publish(tracker)` (không dài thêm; `SessionTracker` không có method → luôn publish như cũ). `tracker.ts` không đụng.
  - Test: Rust `pruned_records_give_the_same_signals`, `view_deltas_carry_only_changed_fields`, deep test thêm nhánh key bị bỏ; 2 test cũ đổi key `a`/`b` → key được giữ (`uuid`/`subtype`…) vì key lạ giờ bị bỏ. TS `tracker-view.test.ts` mới; `tracker-recovery.test.ts` +2 (takeChanged, safety net).
  - Docs: plan §2.3 (delta theo field, T3b CPU, safety net, chưa làm việc share đọc với usage), `.code_index/{core,collector,_meta}.md`, `docs/perf/v87-step4-T3b-win32.json`.
- Tiêu chí 3/3:
  - [x] Bảng node / rust / rust+usage, 2 lượt (dưới).
  - [x] Tổng rust < node rõ: 6.8–6.9 % (rust) và 6.1–6.7 % (rust+usage) so với node 10.1–10.3 % → ≤ 7.6 %.
  - [x] Parity + test + clippy + typecheck sạch.
- Audit / Review:
  - Không comment mới (rg `//` `/*` trên phần thêm = 0), file mới < 400 dòng (`coreTracker.ts` 338).
  - Đổi hành vi (chỉ rust mode, ghi plan §2.3): safety net — sync `skipResolved` của reconcile bị bỏ (không round trip) nếu tracker vừa có sync full / skipResolved < 4 s; sync theo file, catch-up, full luôn chạy, mốc 4 s chỉ tính sync full / skipResolved nên reconcile 5 s luôn qua. Hệ quả: event watcher bị lỡ hiện chậm tối đa ~4 s so với Node. Lý do: đếm 30 s demo 30×6 có 44 sync/s, 31 là reconcile (mỗi lần registry ghi → reconcile cả 30 tracker); sync no-op đo trong Rust 381 µs (Windows: stat/open 20–25 µs, `read_dir` 48 µs). Cần main xác nhận chấp nhận.
  - Prune parse: giá trị sâu > 128 / số ngoài f64 / surrogate lẻ nằm trong key bị bỏ không còn đẩy dòng sang nhánh sanitize / Deep (signals giống hệt, test so với `Value` đầy đủ). Key trùng: key sau thắng như `JSON.parse`.
  - `takeChanged` an toàn vì mọi đầu vào khác của terminal (alias, note, cost, hooks, diff, contexts) đã có đường publish riêng; `applyRecordChange` không publish nhưng record đổi → `takeChanged` true.
- Test (số thật):
  - `cargo test --locked -j 4` toàn workspace pass (core-transcript 38 + 2 ignored; core-bin transcript 4, flow 8, procs 3, pty 16, spike 1, stub 4, core-procs 19, core-pty 35).
  - `cargo clippy --locked -j 4 --all-targets -- -D warnings` Windows + `--target aarch64-apple-darwin` sạch.
  - `npm run typecheck -w @ccm/collector` sạch. `npm test -w @ccm/collector -- --maxWorkers=4`: 41 file pass / 1 skip, 465 pass / 3 skip / 0 fail (lượt trước đó 2 fail timing flaky `core-procs` "half line" + `prompt-search` "stops at 200" 5 s timeout lúc máy tải; chạy riêng 22/22 pass, chạy lại full pass). Parity `tracker-parity*`, `transcript-parity`, `usage-parity`, snapshot `tracker.test.ts` không đổi, pass.
- Đo (Windows, `perf-bench --stage --scenario demo-30x6 --seed 8704`, 120 s sau 20 s settle, core release, % một core; core = mọi con `orrery-core transcript`):

| Lượt | Mode | Collector avg / p95 | Core | Tổng | RSS collector |
|---|---|---|---|---|---|
| base (code T3) | node | 10.0 / 15.3 | — | 10.0 | 108 MB |
| base | rust | 6.1 / 9.2 | 2.2 | 8.3 | 104 MB |
| base | rust+usage | 5.1 / 9.2 | 3.0 (2 con) | 8.1 | 96 MB |
| r1 (delta + takeChanged + prune + 1 open) | rust | 7.0 / 12.1 | 2.9 | 9.9 | 102 MB |
| r1 | rust+usage | 5.2 / 10.7 | 3.1 | 8.3 | 93 MB |
| cuối 1 | node | 10.3 / 17.0 | — | 10.3 | 117 MB |
| cuối 1 | rust | 5.5 / 10.0 | 1.4 | **6.9** | 106 MB |
| cuối 1 | rust+usage | 4.7 / 9.1 | 2.0 | **6.7** | 103 MB |
| cuối 2 | node | 10.1 / 16.9 | — | 10.1 | 105 MB |
| cuối 2 | rust | 5.7 / 10.7 | 1.1 | **6.8** | 102 MB |
| cuối 2 | rust+usage | 4.5 / 9.1 | 1.6 | **6.1** | 98 MB |

  - Core giảm ~2.2–3.0 → 1.1–2.0 chủ yếu nhờ safety net (r1 không thấy khác do máy nhiễu; một lượt lẻ lệch 1–2 điểm). Tổng rust −33 %, rust+usage −37 % so với node.
  - Top còn lại (collector đứng riêng, 30 s, ms/s): rust — usage `ingestLine` 1.1 + `ingestTools` 0.5 (usage Node), `nextTick` 0.9, `store.diff` 0.7, `spawn` 0.7 (process con: git diff khi đổi status, không thuộc tracker), lineReader core 0.7, `routeProjectChange` 0.5; rust+usage — `spawn` 1.2, `diff` 0.8, `buildTerminalOf` 0.6, lineReader 0.6, `routeProjectChange` 0.5, `publish` 0.5. Không còn hàm nào > 1.2 ms/s.
- Ngoài phạm vi / cần main xử lý:
  - Chưa làm "share 1 lần đọc file giữa tracker và usage": hai việc chạy ở 2 con `orrery-core transcript` riêng (usage trên worker thread với offset / debounce / cache v4 riêng); gộp cần 1 process + chuyển dòng giữa main thread và worker — việc lớn, nên tách WP nếu cần. Ghi plan §2.3.
  - Safety net 4 s là đổi hành vi rust mode (xem Audit) — main / chủ nhân chốt.
  - Không đụng `apps/desktop`, `scripts`, `apps/web`, `packages/shared`. `apps/core/target/release/orrery-core.exe` + `apps/desktop/stage` đã build lại. Đo dùng script tạm ngoài repo (`D:/tmp/t3b`), thư mục tạm đã xoá, không còn process tự spawn.
  - Working tree có thay đổi không phải của T3b (`fwp-host.test.ts.snap`, `RELEASE-v2.3.0.md`, `PROGRESS-2026-11-09-v87.md`, ảnh QA) — không đụng.
- Commit message đề xuất: `⚡ v87 step 4 T3b: rust-mode CPU below Node (demo 30x6 collector+core 6.1-6.9 % vs 10.1-10.3 %): pruned record parse (prune.rs), one open per tail read, field-level tracker view delta + mergeView patch, CoreTracker.takeChanged so syncTracker publishes only on change, 4 s safety-net skip for reconcile skip-resolved syncs, perf v87-step4-T3b`
