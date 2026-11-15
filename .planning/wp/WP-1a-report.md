## WP-1a — DONE

- Đã đổi:
  - `apps/core/crates/core-disk/**` (mới): `walk/{mod,win,mac,unix}.rs` (pool luồng std, FindFirstFileExW LARGE_FETCH trên `\\?\`, getattrlistbulk + fallback readdir/lstat, hạ priority, không follow link/junction, không qua ổ khác, cloud-only cờ CLOUD size 0 không mở file, hardlink 1 lần), `index.rs` (node 48 B, string pool + tên đã bỏ dấu, con liên tục, size cộng dồn, merge root), `search.rs` (song song, top N), `fold.rs` (bỏ dấu VN + NFD, glob, nhóm đuôi), `store.rs` (zstd + magic/version, validate khi load), `volumes.rs`, `serve.rs` (Engine + NDJSON), `cli.rs` (serve / bench), `procstat.rs`, `tests/scan.rs`.
  - `apps/core/Cargo.toml` (member), `apps/core/Cargo.lock` (chỉ thêm zstd/zstd-safe/zstd-sys/jobserver, không bump crate cũ), `core-bin/Cargo.toml` + `src/main.rs` (subcommand `disk`).
  - `.code_index/core-disk.md` (mới).
- Tiêu chí: 5/6 đạt, 1 đạt một phần.
  - [x] `cargo test -p core-disk`: 17 pass (size cộng dồn, hardlink 1 lần, junction/symlink không follow, exclude tên + path, cancel lúc đang pause + resume, save/load round-trip + file hỏng, từng filter search, bỏ dấu, path > 300 ký tự + emoji, search song song = tuần tự, ops Engine, stdio).
  - [x] `cargo build -p core-bin` sạch (2 warning có sẵn ở core-desk, không phải của crate này); `cargo clippy -p core-disk --all-targets` 0 warning.
  - [x] Bench thật (bảng dưới).
  - [x] Balanced: CPU tiến trình TB 19.6%, đỉnh 26.2% máy (<= 50%).
  - [x] `.code_index/core-disk.md`: cấu trúc index, ops, invariant.
  - [x] Không comment trong code (rg `//`, `/*` trong src: 0).
  - CHƯA KIỂM: macOS (getattrlistbulk, setiopolicy_np, getmntinfo) và Linux: máy không có target Apple/Linux + C cross compiler cho zstd-sys. Thư mục bị từ chối quyền: không có test tự động (bench C:\Users\dauph gặp 1 denied, quét tiếp bình thường). Ổ rút ra giữa chừng: chưa thử, code đếm vào `errors` và đi tiếp.
- Bench (release, `orrery-core disk bench`):
  - `F:\work` balanced lạnh: 2 549 906 entry, 40.4 s, CPU TB 6.9% / đỉnh 14.2%; đã cache: 12.3 s, TB 19.6% / đỉnh 26.2%. Index 180.5 MB (74 B/entry), RSS peak 323 MB.
  - `C:\Users\dauph` light lạnh: 2 435 809 entry, 104.5 s, CPU TB 1.8%, index 166.2 MB.
  - Search trên 2.5 triệu entry: 3.8-26 ms (chứa "a" 17 ms, `*.mp4` 18 ms, >= 100 MB 3.9 ms, ảnh 30 ngày 4 ms, "tiếng việt" under root 26 ms). Save 32 MB / 1.3 s, load 0.3 s, round-trip ok.
- Ops (`orrery-core disk`, NDJSON như core-desk: `{id,channel,args}` -> `{id,ok,value|error}`, event `{event,payload}`, dòng đầu `{ready,pid,channels}`, `{"type":"shutdown"}`):
  `disk.scan {roots[],level,excludes[],crossMounts,hardlinks,merge=true,save=false}` -> `{started,threads,level}`; `disk.pause|resume|cancel` -> `{ok}`; `disk.status` -> `{scanning,paused,index?}`; `disk.children {path,sort,limit}` -> `{path,size,total,items[]}`; `disk.search {query,minSize,maxSize,after,before,kind,only,under,sort,limit}` -> `{total,ms,items[]}`; `disk.save|load {dir?}`; `disk.clear`; `disk.volumes` -> `[{mount,label,fs,kind,network,total,free}]`. Event `disk:progress` (>= 100 ms/lần) và `disk:done`. Data dir mặc định `CCM_DATA_DIR/disk`.
- Audit / Review / Test:
  - `cargo test -p core-disk -j 18` -> 17 passed.
  - `cargo clippy -p core-disk --all-targets -j 18` -> No issues.
  - `cargo build -p core-bin -j 18` -> Finished; `cargo build --release -p core-bin` -> Finished 2m37s.
  - Smoke NDJSON qua binary: volumes, scan -> disk:done, status đều đúng.
  - FFI: mọi handle bọc Drop (FindClose / CloseHandle / close fd), kiểm INVALID_HANDLE_VALUE + GetLastError, đọc buffer getattrlistbulk có kiểm biên (`read_unaligned` + bound check).
- Ngoài phạm vi cần main xử lý: `.code_index/_meta.md` đang bị sửa (không phải WP-1a, chắc WP-0a); chưa thêm dòng _meta cho core-disk, main thêm 1 dòng trỏ `core-disk.md` nếu muốn. Phase 1b: Electron cần host tiến trình `orrery-core disk` riêng (không đi qua `desk.route`).
- Commit message đề xuất: `feat(core): add core-disk scan engine with in-memory index, search and orrery-core disk ops`
