# WP-v87-6-D3b — báo cáo (2026-11-10)

Port nốt 11 kênh nhóm `previewTools` (`CCM_DESK_PREVIEW_TOOLS`, mặc định `rust`) sang `core-desk`. Logic TS giữ nguyên làm fallback. `replace-files` đạt 1:1 với crate `regress` (không phải để lại TS).

## Đã làm

`ccm:format` · `ccm:check-run` · `ccm:check-cancel` · `ccm:perm-read` · `ccm:perm-allow` · `ccm:sensitive-deny` · `ccm:ctx-sources` · `ccm:memory-doctor` · `ccm:replace-files` · `ccm:http-send` · `ccm:http-cancel`.

- `format`: tool trong project (`node_modules/.bin/prettier`, `.fvm/flutter_sdk/bin/dart`, `ktlint`) đi lên tới root rồi tới PATH, cmd shim Windows (`quote_cmd_arg`, `with_args`), stdin, max 8 MB, deadline 20s, max 2 lượt, ktlint code 1 vẫn lấy text.
- `check-run|cancel`: dòng lệnh lấy từ `detect_scripts` (hoặc `flutter analyze --no-pub`), `parse_check_output` (tsc, tsc pretty, dart / flutter, Kotlin, Vitest, ESLint), max 2 lượt, 1 lượt / folder, deadline 15 phút, cancel theo cờ.
- `perm-read|allow`, `sensitive-deny`: đọc asks của bridge, settings 2 file, `is_risky_perm_rule`, `deny_rules_for`, diff + confirm host (`perm` / `deny`), kiểm file đổi trong lúc confirm (`changed`), backup `.ccm-backup` / `.orrery-bak`, 1 lượt ghi cùng lúc.
- `ctx-sources`, `memory-doctor`: port `contextSources.ts` + `memoryDoctor*.ts`, dùng `extensions::status::{read_configured, configured_json}` của D1.
- `replace-files`: regex JS qua `regress` (lookbehind, cờ `i` / `u`, `\p{L}` cho word), template `$1..$9` `$&`, plan theo dòng, confirm host `replace`, ghi qua `files::write_text_file` (mtime conflict, CRLF, BOM), Local History trước khi ghi (dùng chung `LocalHistory` với `file-write`).
- `http-send|cancel`: `ureq` (không theo redirect, không proxy, max 5 MB body, timeout 30s), phân loại host local / public, confirm 1 lần / host (nhiều request cùng host chờ chung 1 câu trả lời), huỷ theo `id` (request mới cùng `id` huỷ request cũ), header phản hồi sort + gộp như `Headers.forEach` của undici, thêm `accept` / `accept-language` / `user-agent` / `content-type` mặc định như undici.

## File

- Mới (Rust, `apps/core/crates/core-desk/src/preview/`): `spawn.rs`, `format.rs`, `checks.rs`, `perm.rs`, `deny.rs`, `ctx.rs`, `memlint.rs`, `memdoctor.rs`, `replace.rs`, `http.rs`.
- Sửa (Rust): `ipc_tools.rs` (đăng ký 11 kênh), `ipc_files.rs` + `mod.rs` (`LocalHistory` dùng chung), `gate.rs` (`editable_file`), `core-desk/Cargo.toml` (`regress = "0.12.0"`), `Cargo.lock`.
- Sửa (TS): `apps/desktop/src/preview/ipc.ts` (9 kênh từ `ipcMain.handle` sang `tools(...)`), `preview/deskHosts.ts` (`httpHosts`, `ConfirmHost`), `preview/httpSend.ts` (`desk?` + `deskHandle`), `main.ts` (`registerHttpIpc({desk})`).
- Test mới: `apps/desktop/test/desk-preview-run.twin.test.ts` (6 test), `desk-preview-http.twin.test.ts` (3 test). Unit test Rust mới trong từng module.
- `.code_index/{core,desktop,_meta}.md`.
- Không đụng file của D2.

## Số test thật

- `cargo test -j 4 --no-fail-fast` cả workspace `apps/core`: **536 passed, 0 failed, 6 ignored**. Riêng `core-desk` lib: 61 passed (gồm 24 unit test mới ở preview: perm, deny, ctx, memlint, http 4, spawn, format, checks 2, replace 3).
- `cargo clippy -j 4 --all-targets -- -D warnings`: code của gói này sạch; lệnh còn đỏ vì **1 lỗi ở file của D2**: `crates/core-desk/src/capture/files.rs:68` (`manual_pattern_char_comparison`, nên sửa thành `trim_end_matches(['-', '.'])`). Con không sửa file đó. Cần chạy lại clippy sau khi D2 sửa.
- Twin TS vs `orrery-core desk` (debug bin, `CCM_CORE_BIN=apps/core/target/debug/orrery-core`), `twin.logs` và `twin.fellBack` rỗng:
  - `desk-preview-run.twin.test.ts` 6/6: format (prettier / dart / ktlint giả trong project, CRLF, unsupported, fail có detail, missing, text sai kiểu, too-large 4 MB, denied), check (build lỗi tsc + kotlin, ok, unknown-script, flutter, denied, huỷ + busy trên process `make slow` thật), perm-read (asks lọc 7 ngày / cwd / tool), perm-allow (ghi mới, trùng, risky, invalid, quá 20, huỷ confirm, file đổi trong confirm, unreadable, file tab + CRLF + backup; so cả cây `.claude`), sensitive-deny (project / local / user, rm-rf, custom glob, tool sai, scope sai, folder session / không session / root cha, huỷ, file đổi; so cả cây `.claude` + cfg), ctx-sources + memory-doctor (home = ancestor, CLAUDE_CONFIG_DIR riêng, imports vòng, rules, skills, plugins, mcp 3 scope, memory index quá dài / thiếu file / chưa index / thiếu frontmatter / trùng tên / link hỏng / thiếu import / thiếu path / file lớn), replace-files (plain, word + case, regex nhóm `$2$1|$&|$3|$$`, lookbehind, `iu` Unicode, emoji, CJK, `^` rỗng, CRLF, BOM, UTF-8 hỏng, binary, file mất, text cũ `changed`, `no-match`, `(` literal và regex `unsupported`, bad-query, `..`, 501 file, newline trong replacement, dir ngoài root, huỷ confirm, 14 file → `…`; so cây file + confirm call + Local History có ghi).
  - `desk-preview-http.twin.test.ts` 3/3 (server loopback tự dựng, không ra Internet): POST / PUT / DELETE / PATCH, query unicode, header trùng, body tiếng Việt, redirect 302 không theo + 2 `set-cookie`, PNG (binary), BOM, 204, 404, 6 MB → `truncated` size 5 MB, GET + body, HEAD + body, method / URL / header sai, `ECONNREFUSED`, host public bị từ chối (hỏi từng lần), `http-cancel` theo `id`, request mới cùng `id` huỷ request cũ.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: **147 files passed, 1 skipped; 1613 tests passed, 18 skipped**.
- `npm run typecheck`: 0 lỗi (shared, collector, desktop, web).
- Chưa chạy lại twin với `CCM_DESK_PREVIEW_TOOLS=node`: router của twin chỉ đọc env do `startTwin` dựng, không đọc `process.env`; chế độ `node` đã có test TS cũ (`checks.test.ts`, `http-send.test.ts`, `context-sources.test.ts`, `deny-rules.test.ts`…) pass.
- Con có chạy `cargo build -j 4 -p core-bin` (debug, 1 lần, ~2s sau khi `cargo test` đã build) chỉ để twin có `orrery-core` mới. Không chạy build release / `build.mjs` / dist / E2E.

## E2E cần chủ nhân chạy

Build trước: `node apps/desktop/build.mjs` + `cd apps/core && cargo build --release -j 4 -p core-bin`. Instance cô lập (như D3): `CCM_E2E_PORT=4501 CCM_E2E_DBG=9501 CCM_E2E_OUT=<tmp> CCM_CORE_BIN=<bản release copy> node apps/desktop/e2e/<bộ>.cjs`. Bộ liên quan (rg trong `apps/desktop/e2e`): `v70-tools` (http client, replace in files, format), `v72` và `v75` (context dialog, memory doctor), `v76` (`sensitiveDeny`), `file-editor`, `v73`, `v74`. Chạy thêm 1 lần với `CCM_DESK_PREVIEW_TOOLS=node` để so.

## Callback host mới

Không có kênh mới. Thêm 1 giá trị cho `preview.confirm`:

- `preview.confirm {kind: 'http', req: {host, method}}` → `true` nếu user gửi (vỏ gọi `confirmHost(event, host, method)`). Trả lời nằm ở `httpHosts` (`deskHosts.ts`); Rust chỉ gọi khi host là public và chưa được cho phép trong phiên.
- Các kind đã có và giờ dùng thêm: `perm` (`{file, added, diff}`), `deny` (`{file, scope, added, diff}`), `replace` (`{find, replacement, lines, files, paths}`).
- `preview.editableFile` đã có, nay Rust dùng cả `real` lẫn `root` (cho `format`).
- `preview.caps` dùng thêm `confirmPermRules`, `confirmDenyRules`, `confirmReplace`, `claudeConfigDir`, `history` (= có data dir cho asks của `perm-read`).

## Khác biệt còn lại

- `format` / `check-run` / `http-send`: Rust chạy trong process `orrery-core desk` — PATH và env là của process đó (TS dùng env của Electron main). Cùng nguồn khi app khởi động desk (đã kế thừa env).
- `http-send`: `ureq` không có reason phrase của server (dùng `canonical_reason`), không giải nén gzip / br (undici có gửi `accept-encoding`), không có HTTP/2; lỗi mạng chỉ map `ECONNREFUSED` / `ECONNRESET` / `ECONNABORTED` / `ENOTFOUND`, còn lại là text lỗi của `ureq` (khác `cause.code` của undici). Huỷ bằng cờ: request treo ở thread nền tới khi tự timeout (max 30s), kết quả bị bỏ.
- `http-send`: header mặc định thêm khi thiếu: `accept: */*`, `accept-language: *`, `user-agent: node`, `content-type: text/plain;charset=UTF-8` (khi có body). undici còn gửi `sec-fetch-mode`, `accept-encoding`, `connection`.
- `ctx` / `memlint`: sort tên file bằng `locale_cmp` (xấp xỉ ICU `localeCompare`, đúng với tên ASCII); thứ tự `read_dir` khi > 200 thư mục con của `memory-doctor` lấy theo hệ điều hành như TS.
- `perm.rs`: `Date.parse` của asks chỉ nhận ISO 8601 (`core_transcript::time::parse_iso`); bridge ghi `toISOString` nên khớp, dòng định dạng khác bị bỏ.
- `replace-files`: `regress` khác V8 ở các cú pháp hiếm (unicode sets `v`, một số case folding); regress không biên dịch được thì trả `unsupported` thay vì khớp sai.
- Regex port từ JS dùng lớp ký tự rõ ràng (`\s` JS, `\w` ASCII, `\d` `[0-9]`) để khớp V8; `{1,200}` đếm bằng UTF-16 units.
- macOS: các nhánh `cfg(unix)` (process group, `kill(-pid)`, shebang script, `/bin/sh -c`) đã chạy thật trong test; nhánh Windows (`cmd.exe /d /s /c`, `raw_arg`, `taskkill`) chỉ có unit test thuần chuỗi (`quote_cmd_arg`, `cmd_shim`, `with_args`), chưa chạy trên Windows.

STATUS: DONE
