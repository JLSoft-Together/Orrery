# WP-v88-W1 — `currentToolSummary` + `currentToolStartedAt` (collector + core)

## Đã làm
- Sanitize + lấy summary dùng chung, Node và Rust chạy cùng một quy tắc:
  - TS `packages/shared/src/toolSummary.ts` (export ở `index.ts`): `sanitizeToolSummary`, `toolSummaryOf`, `codexToolSummaryOf`.
  - Rust `apps/core/crates/core-transcript/src/toolsum.rs`: `sanitize_tool_summary`, `tool_summary_of`, `codex_tool_summary_of`, `host_home` / `host_platform`.
  - Thứ tự: cắt raw ở 4000 UTF-16, gộp whitespace / newline thành 1 space, `maskSecrets` / `mask_secrets` (home → `~`, `sk-` / `gh*_` / `github_pat_` / `AKIA` / `xox*`, `key=value`, run dài), sau đó `://user:pass@` → `://[masked]@`, `Bearer <x>`, `-password|-passwd|-secret|-token|-api-key <value>`; max 200 UTF-16 rồi thêm `…` (không cắt đôi cặp surrogate; lone surrogate → U+FFFD).
  - Claude: Bash/PowerShell `command`; Read/Edit/Write/NotebookEdit `file_path` (`notebook_path`); Grep `pattern` + ` ` + `path`; Glob `pattern`; Task/Agent `description`; WebFetch URL bỏ query / fragment; WebSearch `query`; MCP / tool khác: không gửi.
  - Codex: `function_call` `cmd` / `command` (mảng `[sh, -lc, x]` → `x`, mảng khác nối bằng space), `apply_patch` (custom `input` hoặc `arguments.input`) → các path `*** Add|Update|Delete File:` nối `, `; MCP (namespace), `local_shell_call`, tool khác: không gửi.
- `mask_secrets` chuyển từ `core-sources/src/mask.rs` sang `core-transcript/src/mask.rs` (core-transcript không phụ thuộc core-sources được); `core_sources::mask` re-export nên core-desk / sampler / prompt search không đổi.
- Node: `transcript.ts` (`tool_use.summary`, `openCalls`, `currentToolSummary()` / `currentToolStartedAt()`), `tracker.ts`, `trackerView.ts` (`StateView`), `codex/rollout.ts` (`tool_call.summary`, `callInfo`), `codex/monitor.ts` `agentOf`. `store.ts` không cần sửa (diff field generic).
- Rust: `signal.rs` / `extract.rs` (`Signal::ToolUse.summary`), `state.rs` (`open_calls`, JSON state), `tracker/view.rs`, `codex/extract.rs` (`ToolCall.summary`), `codex/value.rs` (giữ `arguments` / `input` ở payload), core-monitor `view.rs` / `codex/state.rs` / `codex/publish.rs`, core-store `model.rs` `Agent`.
- `currentToolStartedAt` = `timestamp` của record chứa tool_use đó (chuỗi rỗng bỏ), xoá cùng lúc `currentTool` (tool_result / turn_end / task boundary Codex). Field chỉ hiện khi `currentTool` hiện (main working|waiting, sub / Codex working).
- Docs: `docs/claude-code-integration.md` §5 Current tool + §9 Privacy. Index: `.code_index/{collector,protocol,core,_meta}.md`.

## Test (số thật)
- Bảng ca chung `packages/shared/test/toolSummary.cases.json`: 22 ca sanitize (secret, home, win32 home, newline, cắt UTF-16, emoji ở biên), 20 ca input Claude, 11 ca Codex. TS `apps/collector/test/tool-summary.test.ts` 28 test; Rust `core-transcript/tests/tool_summary.rs` 6 test (cùng bảng) + `core-monitor` `codex::state::keeps_the_call_summary_until_its_output`.
- Parity Node ↔ Rust: `transcript-parity` (state JSON thêm 2 field), `codex-parity` (test mới: exec masked + apply_patch path + xoá khi có output), `agents-twin` (Codex exec qua core-monitor, assert `"currentToolSummary":"cargo test --token [masked]"`), `monitor-twin` (Bash, assert `"echo hi --password [masked]"`), không chứa `hunter22`.
- `cargo test -j 4 --no-fail-fast` cả `apps/core`: 798 passed, 0 failed, 6 ignored (gồm core-desk).
- `cargo clippy -j 4 --all-targets -- -D warnings`: sạch.
- `npm test -w @ccm/collector`: 566 passed, 3 skipped (53 file pass, 1 skip).
- `npm test -w @ccm/shared`: workspace không có script `test` (npm báo Missing script); test của shared nằm ở collector + Rust như trên.
- `npm run typecheck`: exit 0.
- Đã build debug `orrery-core` / bins (không release) để chạy parity.

## Test cũ đã chỉnh vì quy tắc mới (owner chốt lệnh được gửi sau sanitize)
- `codex.test.ts`, `sensitive.test.ts`: trước assert lệnh không bao giờ rời signal; nay assert `summary` đúng và không có field nào khác chứa lệnh.
- `monitor.test.ts`: fixture `echo secret` → `echo hi --token secret` (vẫn assert không lộ `secret`, nay nhờ mask).
- `tracker.test.ts.snap`: thêm 2 key (20 dòng, không đổi gì khác). `sigtests.rs`: tool_use có `summary` với `sk-…` đã che.

## Khác biệt còn lại
- Codex `arguments` có số ngoài range f64 (`1e400`) hoặc lồng sâu > 128: Rust `serde_json` bỏ parse → không có summary, Node vẫn có. Chưa gặp trong rollout thật.
- Đếm `\S{4,}` của rule `key=value` cũ trong `mask_secrets` vẫn tính theo char (lệch UTF-16 với ký tự astral) — có từ trước, không thuộc gói này; rule mới (Bearer) đếm theo UTF-16.
- `local_shell_call` không có summary (J của Codex bỏ nội dung mảng `action.command`).
- DeepSeek không sinh 2 field (ngoài phạm vi).

STATUS: DONE
