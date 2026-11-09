## WP-v87-3-P1 — DONE

- **Đã đổi**: crate mới `apps/core/crates/core-procs` (`table` / `usage` / `cmdline` / `ports`; `win/`, `mac.rs`, `linux.rs`, parser thuần `tcp.rs` / `procfs.rs` / `args.rs` / `start.rs`, `lines.rs`, `deadline.rs`, server `serve.rs`); `orrery-core procs` (`core-bin/src/main.rs`); `core-pty/owners.rs read_pid_table` = `core_procs::table()` (bỏ `/bin/ps`, `parse_ps`; `run_with_deadline` dời sang core-procs); `core-pty/src/hub.rs` 880 dòng → `hub/{mod,from_main,spawn,frames,tests}.rs` (max 261 dòng); test `core-bin/tests/procs.rs`; `.code_index/core.md`. Không thêm crate ngoài (windows-sys / libc / serde có sẵn trong lock); `Cargo.lock` chỉ thêm `core-procs`.
- **Tiêu chí 5/5**: unit + integration (ping, table có core + child `node -e` với ppid = test, usage / cmdline child, ports thấy LISTEN của test, dòng hỏng / > 1 MiB không trả lời, lỗi `op` / `pids`, stdin EOF thoát 0) · so với TS · đo release · owners + parity · clippy 3 target + index.
- **Bảng đo** (release, Windows 11, round trip qua stdin/stdout gồm JSON, 50 lần):

| op | p50 | max | rows |
|---|---|---|---|
| table | 12.3 ms | 15.1 ms | 534 |
| usage 50 pid | 0.37 ms | 0.59 ms | 50 |
| cmdline 50 pid | 0.52 ms | 0.83 ms | 50 |
| ports (v4 + v6) | 0.91 ms | 1.86 ms | 172 |

  RSS process `procs`: working set 5.5 MB, private 1.7 MB (plan ước 2–3 MB: private khớp, working set cao hơn do DLL hệ thống).
- **% khớp với bản TS**: 100% (5/5 lần, 535/535 pid, cùng ppid / name / FILETIME) — script tạm tsx gọi `nativeProcessTable` (koffi) xen kẽ `table` Rust, đã xoá. usage / cmdline / ports chưa so trực tiếp (P3).
- **Khớp 3 giả định của P2**: (1) unix `name`: Linux = `comm` (= `ps comm=`); macOS = basename `proc_pidpath`, rơi về `pbi_name` / `pbi_comm` — `ps comm=` macOS lấy argv0 nên khác ở process đổi argv0 (vd login shell `-zsh` → TS bỏ `-` nên vẫn `zsh`); (2) `startRaw` unix = `ps -o lstart=` UTC, ngày pad 2 ký tự (`Mon Nov  9 07:25:43 2026`), Windows = FILETIME decimal; (3) ports bỏ port 0 (u16 nên không có > 65535). Windows trả mọi state như `parseTcpTable4/6`; macOS / Linux chỉ LISTEN / ESTABLISHED như lsof.
- **Audit**: diff chỉ trong `apps/core/**` + `.code_index/core.md`; không comment; mọi handle Windows bọc RAII `Handle`; buffer NtQuery / TcpTable dùng `Vec<u64>` / `Vec<u32>` (căn hàng), kiểm `len` UNICODE_STRING, max 1 MiB cmdline / 64 MiB bảng TCP; libproc kiểm `got == size`; không `unwrap` trên dữ liệu OS. Ngoài lề: `unix privateBytes` = null (bản TS ps cũng không có).
- **Review**: hub tách thuần cơ học (test + parity giữ xanh). `ports` chạy thread riêng để lsof chậm không chặn `table`; client TS timeout 5 s < deadline lsof 15 s — P2 biết.
- **Test** (kết quả thật): `cargo test --locked -j 4` 87 pass / 3 ignored (vòng 1 con ghi nhầm 107); clippy `-D warnings` sạch Windows, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu` (con thêm target linux qua rustup, chỉ để check). Parity `CCM_PARITY_FULL=1 … test/pty-parity`: 34 pass / 1 skip ở 3 lần liên tiếp; lần chạy đầu tiên báo 1 error (14 pass) không tái hiện được, chưa rõ nguyên nhân.
- **CHƯA KIỂM**: macOS / Linux chưa chạy thật (chỉ type-check + test parser thuần trên Windows).
- **Ngoài phạm vi**: client TS / adapter (P2), parity usage / cmdline / ports + E2E (P3).
- **Commit message đề xuất**: `🦀 core-procs: orrery-core procs NDJSON server (table / usage / cmdline / ports on Windows, macOS libproc + lsof, Linux /proc), owners.rs on core_procs::table, hub.rs split into hub/`

## Vòng 2 (review opus)
- **1 CAO macOS table**: bỏ `proc_listallpids` + `PROC_PIDTBSDINFO` (chỉ đọc được process cùng user). Giờ quét bằng `sysctl(CTL_KERN, KERN_PROC, KERN_PROC_ALL)` như `ps`; `kinfo.rs` là parser thuần đọc `kinfo_proc` 648 byte theo offset (`p_starttime` 0, `p_pid` 40, `p_comm` 243, `e_ppid` 560). Offset con tự tính từ header XNU, cộng lại khớp `sizeof(extern_proc)` = 296. Tên lấy basename `proc_pidpath`, không đọc được thì dùng `p_comm`. Test `cfg(macos)`: có pid 1 (launchd, root = user khác) với ppid 0, và ≥ 95% pid của `ps -A` có trong table. Test này chưa chạy trên Mac thật.
- **2 ports queue**: các `ports` đang chờ trong queue gộp thành 1 lần quét (`next_batch` + `ports_replies`), mỗi id vẫn có reply riêng. Unit test kiểm 3 id chỉ quét 1 lần; integration test gửi dồn 20 `ports`, đủ 20 reply.
- **3 owners**: thêm `core_procs::pid_parents()`, chỉ lấy pid → ppid (Windows chỉ Toolhelp, không mở từng process; macOS kinfo_proc; Linux stat). `owners.rs read_pid_table` dùng hàm này.
- **4**: giữ nguyên thứ tự đọc start sau snapshot; đã ghi vào `.code_index/core.md` là giống bản koffi.
- **Test**: `cargo test --locked -j 4` 90 pass / 3 ignored (vòng 1 đúng ra là 87). Clippy `-D warnings` sạch 3 target (win, aarch64-apple-darwin, x86_64-unknown-linux-gnu). Parity pty 34 pass / 1 skip, chạy 2 lần đều vậy. Không build `--release` (P3 đang dùng); bảng đo vẫn là số của vòng 1.