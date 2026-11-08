## WP-perf-1 — DONE
- Đã đổi:
  - apps/collector/src/usage.ts:408-422 — ingest gom mảnh chưa có newline vào mảng, `Buffer.concat` một lần khi gặp newline; `highWaterMark` 1 MB; `consumed` vẫn chỉ tính dòng có newline.
  - apps/collector/src/office/hub.ts — option `idleCommitMs` (5000): 0 subscriber thì commit mỗi 5 s, có subscriber thì 200 ms như cũ; subscriber đầu tiên huỷ timer idle đang chờ và lên lịch nhịp nhanh; `subscribe` vẫn `commit(true, clientId)` ngay.
  - apps/collector/src/gitstat.ts — `createDiffStatReader(run, opts)`: dedupe in-flight theo cwd, cache kết quả repo 3 s, cache "không phải repo" (null) 5 phút, key lowercase trên win32; `readDiffStat` = reader mặc định. monitor.ts không phải sửa (vẫn gọi `deps.diffStat`, kết quả gán cho mọi terminal cùng cwd).
  - Test: test/usage.test.ts (dòng 70 KB / 1.3 MB / 3 MB + dòng cuối dở rồi nối tiếp), test/office-hub.test.ts (sửa test cũ sang nhịp 5 s; thêm 2 test), test/gitstat.test.ts (mới, 3 test).
- Tiêu chí: 5/5 đạt.
- Audit: không code chết, không comment mới, giữ CRLF. Contract `entry.offset` không đổi.
- Review: edge: chunk không có newline (dòng > 1 MB) được gom đúng; dòng cuối dở không tính vào offset; subscriber mới nhận state ngay; thay đổi store khi 0 subscriber được ghi timeline trễ tối đa 5 s (recorder tự throttle, nên timelapse 7 ngày vẫn đúng). Lỗi git/timeout tạm thời cũng bị cache 5 phút như "không phải repo".
- Test: `npm test -w @ccm/collector` → 408 pass / 0 fail (1 skipped sẵn có); `npm run typecheck -w @ccm/collector` → pass.
- Đo trước/sau (script tạm, vòng đọc cũ vs mới, file 580 MB, 200 dòng 1-3 MB + 60k dòng 3 KB): trước 10.36 CPU-s / 26.4 s wall; sau 0.84 CPU-s / 0.42 s wall; kết quả byte/dòng giống hệt. Chưa đo trên toàn UsageIndex (47.7 CPU-s gốc gồm cả JSON.parse).
- Hot file cần thêm / Ngoài phạm vi: appendFileSync của timeline nằm trong packages/habblaud/server/history/timeline.ts (ngoài phạm vi, không đổi; đã giảm tần suất ingest khi idle). CHƯA KIỂM: đo thực tế office hub 30x6 và số spawn git trên app thật.
- Commit message đề xuất: perf(collector): linear usage scan, idle office commit cadence, per-cwd git diffstat dedupe

## Vòng 2 (gitstat)
- apps/collector/src/gitstat.ts: bỏ hit cache 3 s. Coalesce theo cwd: có lượt in-flight thì request mới không chạy song song, chia sẻ đúng 1 lượt kế (chạy sau khi lượt hiện tại xong), mọi caller đến trong lúc chờ nhận kết quả lượt sau (mới nhất).
- `git()` phân biệt: exit code 128 + stderr "not a git repository" → `NOT_A_REPO` (export); lỗi khác (timeout, git lỗi, chưa có git) → null. Chỉ `NOT_A_REPO` được cache 5 phút (`notRepoTtlMs`); lỗi khác không cache, lần sau chạy lại. Chữ ký `readDiffStat` giữ nguyên.
- test/gitstat.test.ts (5 test): không chạy song song và tối đa 2 lượt cho 3 caller đồng thời; không cache kết quả repo; edit trong lúc in-flight → lượt cuối thấy dữ liệu mới, đúng 2 lượt; non-repo cache theo TTL; timeout/lỗi khác không cache.
- `npm test -w @ccm/collector` → 410 pass / 0 fail (1 skipped); `npm run typecheck -w @ccm/collector` → pass.
- Lưu ý: file Temp `g.mjs` thuộc agent khác, con không ghi đè; script tạm của con đã xoá.
