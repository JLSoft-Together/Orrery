# HANDOFF — Orrery v87 bước 4 xong (2026-11-10, Windows)

Thay `.planning/HANDOFF-2026-11-10-v87-mac.md`. Nhật ký: `.planning/PROGRESS-2026-11-09-v87.md` (Đợt 5). Ràng buộc giữ như §4 handoff 2035 (commit + push khi WP đạt, không trailer, không release, máy ≤ 80 %).

## Đã xong (đã push)
- T4 Windows (`52bce4a`): collector + core demo 30×6 6.54 % (Node 10.7 %), cổng ≤ 7.6 % ĐẠT; E2E 56 bộ pass sau sửa 2 script (v70 chờ http body, v51 tắt `compactAuto`).
- T5a (`6721d74`): Codex rollout đọc bằng Rust sau `CCM_CORE_CODEX` (mặc định node).
- T5b (`e97776d`): hooks / statusline / DeepSeek outbox bằng Rust sau `CCM_CORE_SIDEFILES` (mặc định node).
- Codex làm tốt việc đo / test (T1g, T4w); việc code nhiều file (T5b) Codex báo cáo sai → giao Claude.

## Việc tiếp (chờ chủ nhân chốt)
1. Đổi mặc định `CCM_CORE_CODEX` + `CCM_CORE_SIDEFILES` sang `rust` sau khi chạy app thật vài ngày (QA tay: Codex dưới agent Claude, compact chip, statusline limits, DeepSeek nếu bật).
2. Bước 5 (`PLAN-v87-rust-core.md` §6): store + diff + WS server + gitstat / favorites / aliases / history / toast sang Rust, bỏ Node collector — 4–6 tuần, cần plan con + grill trước.
3. (Tuỳ) mac: port nốt E2E script Windows (`QA-v87-mac-result.md` mục T4).
