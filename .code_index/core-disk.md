# core-disk (v101 phase 1a)

Rust crate `apps/core/crates/core-disk`: quét ổ / thư mục, giữ index trong RAM, lưu đĩa nén, tìm kiếm. Chạy bằng `orrery-core disk` (NDJSON stdio) hoặc `orrery-core disk bench <path>`. Electron host: `apps/desktop/src/diskMap/` (phase 1b, xem `desktop.md` mục diskMap).

## File

| File | Vai trò |
|---|---|
| `src/walk/mod.rs` | `scan(opts, control, progress)`: pool luồng std (stack job dùng chung `Mutex<Queue>` + `Condvar`), `Level` (light 2 luồng, balanced nửa số nhân, max đủ nhân), `Control` (pause / resume / cancel), `Progress` (atomic files / dirs / bytes / denied / errors + thư mục đang đi), `Excludes`, `build()` dựng index BFS |
| `src/walk/win.rs` | `FindFirstFileExW(FindExInfoBasic, FIND_FIRST_EX_LARGE_FETCH)` trên path `\\?\`, size / attr / mtime lấy trong entry; `lower_priority` (`THREAD_MODE_BACKGROUND_BEGIN`, max = `THREAD_PRIORITY_BELOW_NORMAL`); `cluster_size`; `normalize_root` |
| `src/walk/mac.rs` | `getattrlistbulk` (name, devid, objtype, modtime, flags, fileid, linkcount, allocsize, datalength); `ENOTSUP` / `EINVAL` ở lần gọi đầu thì trả `None` để rơi về `readdir` |
| `src/walk/unix.rs` | fallback `read_dir` + `symlink_metadata` (`st_blocks * 512`, `SF_DATALESS`), `setiopolicy_np` + `PRIO_DARWIN_BG` trên macOS |
| `src/index.rs` | `Node`, `Index`, `Builder`, `merge`, `find`, `path`, `sorted`, `clean_path`, `under` |
| `src/search.rs` | `Query`, `search()` song song theo chunk |
| `src/fold.rs` | bỏ dấu (bảng Latin + tiếng Việt, bỏ dấu kết hợp U+0300..036F cho tên NFD của macOS), `contains_ci`, `glob_ci`, `cmp_ci`, nhóm đuôi file |
| `src/store.rs` | `save` / `load` / `encode` / `decode` |
| `src/volumes.rs` | danh sách ổ |
| `src/serve.rs` | `Engine` + `dispatch` + vòng NDJSON |
| `src/cli.rs` | `run(args)`: `serve` (mặc định) hoặc `bench` |
| `src/procstat.rs` | CPU time + RSS của tiến trình (cho bench) |
| `tests/scan.rs` | test trên cây thư mục tạm |

## Cấu trúc index

- `nodes: Vec<Node>` 48 B/node: `parent u32` (`NONE` = root), `name_off u32` + `name_len u16` vào `pool`, `fold_len u16` (tên đã bỏ dấu + chữ thường nằm ngay sau tên gốc trong pool, chỉ có khi tên không phải ASCII), `flags u16`, `group u8` (nhóm đuôi), `first u32` + `count u32` (con là một khoảng liên tục), `size u64` (trên đĩa, thư mục = cộng dồn), `len u64` (logic, thư mục = cộng dồn), `mtime u32` (giây unix).
- Thứ tự BFS: con luôn đứng sau cha, nên cộng dồn size chỉ cần 1 vòng ngược (`sum_sizes`).
- Root node có `name` = đường dẫn hiển thị đầy đủ (`C:\Users\x`, không `\\?\`); `path(i)` ghép tên theo chuỗi `parent`.
- `flags`: DIR, LINK, CLOUD, DENIED, HARDLINK (bản lặp, size 0), HIDDEN, SYSTEM, MOUNT (thư mục khác ổ, không vào), ERROR.
- Đo thật: 71-74 B/entry tính cả pool (2.5 triệu entry = 180 MB). Lúc dựng có thêm listing tạm nên peak RSS khoảng 1.8 lần index.
- Scan mới mặc định `merge`: root cũ không chồng lấn root mới thì giữ (copy subtree qua `Builder::append_tree`); root mới nằm trong root cũ thì chỉ thay subtree đó (`append_grafted`, size / len các thư mục cha điều chỉnh theo chênh lệch, root cũ giữ nguyên tên); root mới chứa hoặc trùng root cũ thì root cũ bị thay. Root mới nằm trong root cũ nhưng path không có trong index cũ (bị exclude / mới tạo) thì rơi về thay cả root cũ (`merge_replace`).
- Giới hạn index: `Builder` dừng khi node >= u32::MAX - 1 hoặc pool > u32::MAX byte (`overflowed()`), scan trả `ScanError::TooLarge`; `Index::can_merge` kiểm tổng old + new, vượt thì `disk:done {ok:false, error}` và giữ index cũ.

## Invariant

- **Không follow link**: Windows mọi thư mục có reparse point (junction, mount point, symlink...) trừ tag cloud (`tag & 0xFFFF0FFF == 0x9000001A`, OneDrive) đều ghi LINK, size 0, không vào. File reparse thuộc tag link (symlink, AppExecLink, LX symlink, AF_UNIX) cũng LINK; tag khác (dedup...) vẫn là file thường. macOS / Linux dùng `lstat` / objtype `VLNK`.
- **Không qua ổ khác**: Unix so `dev` của thư mục con với `dev` của root, khác thì cờ MOUNT, trừ khi `crossMounts`. Windows mount point là reparse nên đã bị chặn bởi luật trên.
- **Không mở file cloud-only**: Windows `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` / `RECALL_ON_OPEN` / `OFFLINE`, macOS `SF_DATALESS` thì cờ CLOUD, size trên đĩa 0, không gọi `CreateFileW` / `GetCompressedFileSizeW` (chỉ đọc attribute có sẵn trong entry thư mục). Reparse tag cloud (`is_cloud_tag`) cũng tính là CLOUD dù thiếu bit attribute. Thư mục CLOUD vẫn nằm trong index nhưng **không đi vào** (không tải listing qua mạng), không gắn MOUNT. `win::classify(attrs, tag)` gom luật này.
- **Size trên đĩa**: Windows làm tròn lên cluster của volume; file COMPRESSED / SPARSE mới gọi `GetCompressedFileSizeW` (đắt, nên chỉ khi cần). File nhỏ nằm trong MFT bị tính dư 1 cluster, chấp nhận. macOS `ATTR_FILE_ALLOCSIZE` / `st_blocks * 512`.
- **Hardlink tính 1 lần** theo `(dev, ino)` khi `nlink > 1`. Unix bật mặc định (miễn phí); Windows tắt mặc định vì phải mở handle mỗi file (`FILE_READ_ATTRIBUTES` + `OPEN_REPARSE_POINT`, không đọc nội dung), bật bằng `hardlinks: true`. Bản nào được tính là bản quét tới trước (không cố định).
- Lỗi quyền / path mất / lỗi khác: đếm (`denied`, `errors`), gắn cờ lên node thư mục, quét tiếp. Path Windows luôn có `\\?\` nên > 260 ký tự vẫn đi được. Tên không hợp lệ (UTF-16 lẻ surrogate trên Windows, byte không phải UTF-8 trên Unix): index lưu bản lossy, tên gốc giữ trong `alts: Vec<OsString>` của listing (`Raw.alt` = chỉ số, `NO_ALT` = không có) để dựng path con đi vào đúng thư mục.
- Thư mục con không được là link tại lúc mở: Unix `O_NOFOLLOW` (mac bulk mở với cờ này, fallback `read_dir` kiểm bằng open `O_NOFOLLOW` trước, còn khe hở nhỏ giữa open và `read_dir`); Windows `GetFileAttributesW` kiểm lại reparse trước khi list (`DirCtx.root` = true thì bỏ qua). Bị link giữa chừng thì đếm `errors`.
- Panic: `WorkerGuard` trả `busy` + đặt `done` khi worker panic, scan trả `ScanError::Panicked` thay vì treo. `Engine::run_scan` bọc `catch_unwind`: panic thì vẫn trả slot scan, gửi `disk:done {ok:false, error}`; `Reporter` có `Drop` nên thread progress luôn dừng.
- Cancel: worker kiểm trước mỗi job, scan trả `ScanError::Cancelled` và giữ nguyên index cũ. Pause chặn worker trước job kế (job đang chạy làm nốt 1 thư mục).
- Progress: reporter thread gửi `disk:progress` cách nhau >= 100 ms (max 10 lần/giây), dừng ngay khi scan xong.
- Exclude: chuỗi không có `/` `\` = luật tên (glob `*` `?`, không phân biệt hoa thường trên Windows / macOS), có dấu phân cách = luật path (áp cho cả thư mục lẫn file). Mục bị loại không có trong index.
- Ổ mạng trong `disk.volumes`: không gọi `GetVolumeInformationW` / `GetDiskFreeSpaceExW` vì có thể treo khi mạng chập chờn, total / free = 0.

## Tìm kiếm

- `query` tách theo khoảng trắng, mọi term phải khớp; term có `*` `?` là glob khớp cả tên, còn lại là chuỗi con. Term được bỏ dấu + chữ thường; tên ASCII so không phân biệt hoa thường trực tiếp trên pool, tên có dấu so trên bản đã bỏ dấu.
- Filter: `minSize` / `maxSize` (size trên đĩa, file cloud = 0), `after` / `before` (ms), `kind` (image / video / audio / doc / archive / installer / code), `only` (file / dir), `under` (path trong index: chỉ duyệt subtree), `sort` (size / name / mtime), `limit` (max 10 000). Root không bao giờ là kết quả.
- Từ 200 000 ứng viên trở lên chạy song song nửa số nhân, chunk 32 768, mỗi luồng giữ top N bằng `select_nth_unstable`, cuối cùng gộp + sort. Trả `total` = tổng số khớp.

## Forget (`Index::forget`)

- Index bất biến sau khi dựng (con là khoảng liên tục, BFS), nên forget dựng lại index mới bằng `Builder::append_pruned`: copy theo BFS, bỏ node trong tập xoá, cộng `count` lại, trừ delta size / len đã tính trước cho từng tổ tiên (`HashMap`). Path lồng nhau chỉ tính path ngoài cùng. O(n), không đọc đĩa.
- `Engine::forget` giữ write lock của index suốt lúc dựng lại, nên `disk.children` / `disk.search` chờ (vài trăm ms với 2.5 triệu entry). Scan đang chạy đọc index cũ để merge lúc xong: forget xen giữa lúc scan đọc và lúc set index có thể bị mất cho root không trùng (hiếm, quét lại là đúng).
- Không tự save; host gọi `disk.save` sau forget.

## Lưu đĩa

`<dir>/index.bin`: `ORRDISK\0` + version u32 (=1) + zstd level 3 của: 7 x u64 stats, roots, số node, độ dài pool, node 43 B little-endian, pool. `decode` kiểm UTF-8, biên ký tự, `parent < i`, `first > i`, khoảng con trong mảng, root hợp lệ. `load` giải nén streaming max 4 GiB (`MAX_DECODED`), vượt thì lỗi `too large`. Ghi file tạm riêng mỗi lần (`index.bin.<pid>.<seq>.tmp`), khoá save toàn tiến trình, `sync_all` rồi rename, lỗi thì xoá tmp. `dir` mặc định `CCM_DATA_DIR/disk`, truyền `dir` để đổi.

## Ops (`orrery-core disk`, NDJSON giống `core-desk::stdio`)

Vào: `{"id","channel","args"}` (`args` là mảng thì lấy phần tử đầu), `{"type":"shutdown"}`. Ra: dòng đầu `{"ready":true,"pid","channels"}`, trả lời `{"id","ok":true,"value"}` / `{"id","ok":false,"error"}`, sự kiện `{"event","payload"}`. Mỗi request chạy thread riêng (pause / cancel không bị scan chặn), panic trong request trả `{ok:false, error:"internal error"}`. Đọc dòng bằng `read_until` + `from_utf8_lossy` nên byte lỗi không làm chết serve; dòng không phải object bị bỏ, dòng có `id` mà `id` không phải u64 hoặc thiếu `channel` string thì trả `{id, ok:false, error}` (id gốc được echo). Hết input / `shutdown`: cancel scan rồi đợi tối đa 3 s cho request + scan đang chạy gửi nốt reply / `disk:done`.

| Channel | Args | Value |
|---|---|---|
| `disk.scan` | `roots[]`, `level`, `excludes[]`, `crossMounts`, `hardlinks`, `merge` (true), `save` (false) | `{started, threads, level}` ngay; đang quét thì lỗi `busy` |
| `disk.pause` / `disk.resume` / `disk.cancel` | - | `{ok}` (false nếu không có scan) |
| `disk.status` | - | `{scanning, paused, index?: summary}` |
| `disk.children` | `path` (rỗng = các root), `sort`, `limit` (500) | `{path, size, total, items[]}` |
| `disk.search` | xem mục Tìm kiếm + `query` | `{total, ms, items[]}` |
| `disk.save` / `disk.load` | `dir?` | `{path, bytes, ms}` / `{loaded, ...summary, loadMs}` |
| `disk.clear` | - | `{ok}` |
| `disk.forget` | `paths[]` | `{removed, freed, entries}`: bỏ node (cả cây con) khỏi index, trừ size / len lên mọi thư mục cha, root bị forget thì bỏ root; path không có trong index bị bỏ qua; không có gì để bỏ thì giữ nguyên index (`removed: 0`) |
| `disk.mft` | `drive` (`C`, `c:`, `C:\`), `merge` (true), `save` (false) | `{started, mft, root}` ngay; `unsupported` / `busy` / `drive required`; kết quả qua `disk:done` (xem mục MFT) |
| `disk.volumes` | - | `[{mount, label, fs, kind: fixed/removable/network/cdrom/ram/unknown, network, total, free}]` |

Item: `{path, name, dir, size, len, mtime(ms), count, kind, cloud, link, hardlink, denied, mount, hidden}`. Summary: `{entries, files, dirs, bytes, denied, errors, ms, scannedAt, roots[], indexBytes}`.

Sự kiện: `disk:progress` `{files, dirs, bytes, denied, errors, current, paused, elapsedMs}`; `disk:done` `{ok:true, ...summary, scanMs, saved?|saveError?}` hoặc `{ok:false, cancelled:true}` / `{ok:false, error}`. Slot scan được trả trước khi gửi `disk:done`, nên nhận `disk:done` xong là quét tiếp được.

## Quét siêu nhanh MFT (WP-1d, Windows NTFS)

| File | Vai trò |
|---|---|
| `src/mft/record.rs` | `apply_fixup` (stride 512, kiểm USN từng sector), `parse` / `parse_fixed` -> `Parsed` (in_use, dir, seq, base ref, `$STANDARD_INFORMATION` mtime + attr, `$FILE_NAME` parent ref + seq + namespace, `$DATA` không tên: resident = (0, len), non-resident chỉ lấy extent VCN 0 = (allocated hoặc compressed size khi cờ compressed / sparse, data size), `$REPARSE_POINT` tag, `$ATTRIBUTE_LIST` có / không, vị trí runlist), `data_runs` |
| `src/mft/tree.rs` | `Table`: mảng theo số record (size, len, mtime, flags, seq, đầu danh sách tên) + danh sách tên liên kết + pool; record extension (base != 0) gộp tên / data vào record gốc; `build(root_name)` -> `Index` |
| `src/mft/wire.rs` | kiểm tham số helper, HELLO + token, frame `[kind u8][len u32][payload]` (progress 16 B, error max 4096 B, index max 3 GiB) |
| `src/mft/mod.rs` | `Source` (`MemorySource` cho fixture), `collect`, `produce` (helper: progress mỗi 100 ms, index = zstd level 1 của `store::encode`), `consume` / `accept` (tiến trình thường: inflate max 4 GiB, `store::decode` có kiểm cấu trúc, root phải đúng `X:\`), `drive_letter`, `run` / `supported` |
| `src/mft/win.rs` | pipe server, launch UAC, client, đọc volume, `retrieval_pointers` |
| `src/mft/sample.rs` | dựng record FILE đúng format cho test + fixture `tests/fixtures/mft-sample.bin` (31 record) |

Luồng: op `disk.mft {drive, merge?, save?}` (tiến trình thường) -> kiểm `supported` (GetDriveTypeW fixed / removable + GetVolumeInformationW `NTFS`, không thì lỗi `unsupported`; ổ mạng, FAT, exFAT, ReFS, BitLocker đang khoá đều rơi vào đây) -> lấy slot scan (`busy` nếu đang quét) -> trả `{started, mft, root}` ngay -> thread: tạo pipe, `ShellExecuteExW` verb `runas` với `current_exe()` và đúng 3 tham số `disk mft-helper <X> <pipe> <token>` -> nhận stream -> `install` (merge / save giống scan thường) -> `disk:done {..., mft:true}`. Progress `disk:progress {files: records, phase:"mft", records, total, current: "X:\"}`.

Invariant bảo mật:
- Helper elevated (`cli` `mft-helper`) chỉ nhận đúng 3 tham số: ổ `[A-Z]` (chữ hoa, 1 ký tự), pipe `\\.\pipe\orrery-mft-<32 hex thường>`, token 64 hex thường; sai bất kỳ -> exit 2. Không đọc stdin, không ghi file, không chạy lệnh khác; thread deadline 10 phút gọi `process::exit(4)`.
- Pipe do tiến trình thường tạo: tên ngẫu nhiên 128 bit + token 256 bit (`BCryptGenRandom`), DACL `D:P(A;;GA;;;<SID user>)`, `FILE_FLAG_FIRST_PIPE_INSTANCE` (tên đã có ai tạo thì lỗi, không launch), 1 instance, `PIPE_REJECT_REMOTE_CLIENTS`, overlapped để mọi chờ có max + cancel.
- Server: `ConnectNamedPipe` chờ cùng handle tiến trình helper (helper chết trước khi nối -> lỗi `helper exited (code)`), rồi `GetNamedPipeClientProcessId` phải bằng PID tiến trình vừa launch (giữ handle nên PID không bị tái dùng), rồi đọc HELLO + token (max 30 s, so sánh không rẽ nhánh theo byte), sai -> đóng pipe, lỗi `bad token`; đúng mới gửi ACK `0x06`.
- Client (helper): mở pipe với `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION` (server không impersonate được token admin), gửi HELLO + token trước, không nhận đúng ACK -> exit 3, không đọc gì khác từ pipe.
- UAC bấm Không: `ShellExecuteExW` lỗi `ERROR_CANCELLED` -> `disk:done {ok:false, error:"elevation-declined"}`, không thử lại.
- Cancel: đóng pipe (không TerminateProcess được tiến trình admin), helper ghi lỗi thì thoát. Helper chết giữa chừng: pipe gãy -> `helper exited`. Timeout tổng 10 phút cả hai phía.
- `mft-fixture <X> <pipe> <token> <file>` cho test / E2E: từ chối chạy khi tiến trình đang elevated (exit 5), file max 256 MiB. Engine chỉ dùng fixture khi env `CCM_MFT_FIXTURE` có giá trị (đặt ở tiến trình main, trang web không đổi được); có fixture thì bỏ qua kiểm NTFS và launch bằng `Command` thường.
- Record trỏ quá số record của volume (base ref, parent ref >= tổng record theo `MftValidDataLength`) bị bỏ (`Table.skipped`), không resize mảng theo số lạ.

Luật dựng cây:
- Bỏ record 0..15 trừ root 5 (`$MFT`, `$Extend`... không hiện; con của `$Extend` không đi tới được nên cũng không hiện). Record không in-use bỏ. Fixup sai -> `bad` + bỏ.
- Tên: bỏ namespace DOS (2) khi có tên khác (Win32 / POSIX / Win32+DOS); tên trùng parent + text bỏ bản lặp. Tên có parent seq khác seq hiện tại của record cha (cha đã bị xoá và tái dùng) bị bỏ.
- Hardlink (nhiều tên Win32 / POSIX): bản gặp đầu theo BFS giữ size + len, bản sau cờ HARDLINK, size 0, len 0.
- CLOUD (attr OFFLINE / RECALL_*, hoặc tag cloud): size 0, len giữ. Khác walker thường: thư mục cloud **có** đi vào, vì MFT là dữ liệu local, không tải gì qua mạng.
- LINK (tag symlink / junction / AppExecLink / LX / AF_UNIX): size 0, thư mục không đi vào. Mỗi thư mục chỉ vào 1 lần (`entered`), nên dữ liệu MFT hỏng có vòng lặp cũng dừng.
- Size: non-resident = allocated (compressed size khi compressed / sparse); resident = 0 (nằm trong MFT), walker thường làm tròn lên 1 cluster nên tổng MFT nhỏ hơn một chút. Không tính stream có tên (ADS), giống walker. Thư mục size riêng 0.
- `$MFT` của volume: đọc record 0 tại `MftStartLcn`, runlist của `$DATA`; runlist không phủ hết `MftValidDataLength` (có `$ATTRIBUTE_LIST`, MFT phân mảnh nhiều) -> `FSCTL_GET_RETRIEVAL_POINTERS` trên `X:\$MFT`, vẫn thiếu -> lỗi `fragmented $MFT`. Đọc chunk 4 MiB, độ dài làm tròn lên sector.
- RAM helper ước tính cho 10 triệu record: ~320 MB bảng + ~160 MB tên + pool + mảng con (~250 MB) + `Index`. Tiến trình thường chỉ giữ blob zstd + `Index` sau decode. Giới hạn u32 record (index `Builder` `TooLarge`).

Test: `tests/mft.rs` (parse, fixup, runlist, cây, tham số, frame, fixture file khớp `sample::image()`, cập nhật bằng `UPDATE_MFT_FIXTURE=1`), `tests/mft_pipe.rs` (`harness = false`: binary test tự làm helper khi nhận `disk ...`, đi đủ pipe + PID + token + nạp index qua `Engine::with_mft_fixture`), unit trong `win.rs` (token sai, PID sai, ACK sai, tên pipe trùng, cancel). Chạy thật có UAC: `node scripts/mft-smoke.mjs [C] [--no-compare] [--level=...]` (chủ nhân chạy tay, in thời gian, số entry, so size với scan thường).

## Bench (15/11/2026, release, Windows 11, 20 luồng)

| Thư mục | Level | Entry | Thời gian | CPU tiến trình | Index RAM | Search |
|---|---|---|---|---|---|---|
| `F:\work` (lạnh) | balanced | 2 549 906 | 40.4 s | TB 6.9%, đỉnh 14.2% máy | 180.5 MB | 3.9-26 ms |
| `F:\work` (đã cache) | balanced | 2 549 906 | 12.3 s | TB 19.6%, đỉnh 26.2% máy | 180.5 MB | |
| `C:\Users\dauph` (lạnh) | light | 2 435 809 | 104.5 s | TB 1.8%, đỉnh 5.9% máy | 166.2 MB | 3.8-28 ms |

Save 32 MB / 1.3 s, load 0.3 s. `THREAD_MODE_BACKGROUND_BEGIN` hạ IO priority rất mạnh nên lần quét lạnh chậm; muốn nhanh hơn có thể cho balanced dùng `BELOW_NORMAL` (đổi đánh đổi máy mượt) hoặc chuyển walker sang `GetFileInformationByHandleEx(FileIdExtdDirectoryInfo)` (có sẵn AllocationSize + FileId, bỏ được phép làm tròn cluster và mở handle cho hardlink).

## Chưa kiểm

- macOS (`getattrlistbulk`, `setiopolicy_np`, `getmntinfo`) và Linux: chưa compile được trên máy này (không có target Apple / cross C compiler cho zstd-sys). Phần mới chưa kiểm trên macOS: buffer bulk thread_local dùng lại, `failed` khi entry sai, `checked_add` offset, `O_NOFOLLOW`, `alts` cho tên không phải UTF-8.
