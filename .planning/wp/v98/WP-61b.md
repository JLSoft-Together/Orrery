## WP-61b: web app — sửa file, git stage / unstage / commit, Run (configs / devices / start / stop / reload)

- Mục tiêu: viewer của web app (`apps/pwa`) làm được như app Android: sửa + lưu file, stage / unstage / commit, chạy / dừng / reload app qua Run.
- Spec: `.planning/PLAN-v98-ios-pwa.md`; việc dở ở `.planning/wp/v98/WP-55b-report.md` "Chưa làm: sửa file (fs.write), stage / commit / branch, run.start". Docs: `docs/remote-integration.md` (§ `fs.*`, `git.*`, `run.*`, xác nhận nguy hiểm), `.code_index/web-pwa.md`.
- Mẫu tham chiếu (đã chạy): Android `apps/mobile/lib/**` dùng `fs.write`, `fs.files`, `fs.changed`, `git.stage`, `git.unstage`, `git.commit`, `git.discard`, `run.configs`, `run.devices`, `run.start`, `run.stop`, `run.reload`. Handler desktop `apps/shell/src/remote/**` đã hỗ trợ đủ — không đổi hợp đồng; nếu thấy thiếu gì phía desktop thì ghi báo cáo, không tự đổi.
- Code: `apps/pwa/src/{components/Viewer*.vue,components/CodeLines.vue,components/DiffLines.vue,lib/viewerModel.ts,lib/viewerApi.ts,i18n/{en,vi}.ts}` + component mới nếu cần (≤ ~400 dòng / file).
- Phạm vi được đụng: các chỗ trên + test + e2e `apps/pwa/e2e/**` (peer `e2e/peer` thêm handler giả cho lệnh mới), `.code_index/web-pwa.md`, `docs/qa/v98/WP-61b/**`, báo cáo. Cấm đụng: `apps/shell/**`, `apps/relay/**`, `apps/web/**`, `apps/mobile/**`, phần Workspace / pane (WP-61a đã làm).
- Model: sonnet. Chạy sau WP-61a.

### Việc
1. Sửa file: nút Sửa trong viewer file → editor textarea mono (không highlight cũng được), Lưu gửi `fs.write` kèm mốc phiên bản như Android (chống ghi đè khi file đổi trên đĩa: hiện xung đột, cho tải lại); thoát khi chưa lưu thì hỏi. Chế độ chỉ xem → ẩn nút.
2. Git (màn Changes): stage / unstage từng file và tất cả, ô commit message + Commit (`git.commit`), discard có xác nhận nguy hiểm (đã có thì giữ). Cập nhật danh sách qua `git.watch` / `git.changed` hiện có.
3. Run: danh sách cấu hình `run.configs`, chọn thiết bị `run.devices`, Start / Stop / Reload, log theo `run.event` (đã có viewer log thì nối vào).
4. Xác nhận nguy hiểm đi qua `confirm.ask` của desktop như Android; lỗi `denied` → thông báo chỉ xem.
5. i18n `en` + `vi` (GLOSSARY: giữ `stage`, `commit`, `run`, `reload`; không em dash, không chấm giữa). Motion khi danh sách đổi.
6. Test: vitest (model editor / xung đột / git / run), E2E WebKit + Chromium: sửa + lưu, xung đột, stage + commit, discard xác nhận, run start / stop. Ảnh phone + iPad sáng / tối `docs/qa/v98/WP-61b/`.

### Tiêu chí nghiệm thu
- [ ] typecheck sạch; `npm test -w @ccm/pwa` pass (số); `npm run e2e -w @ccm/pwa` pass 2 engine (số); 0 vi phạm CSP; build ok.
- [ ] Không file mới > ~400 dòng. Bước QA iPhone cho chủ nhân.

### Luật (bắt buộc)
- Không viết comment trong code (mọi loại, kể cả TODO); lý do ghi báo cáo hoặc `.code_index/`.
- Không `git commit` / `push`. Không deploy relay.
- Tool nặng ≤ 80% CPU / RAM; trước khi build peer Rust (`cargo`) kiểm `tasklist | grep -i cargo`, dùng `-j 4`. Không dùng cổng 4320–4322.
- E2E pwa ghi đè ảnh `docs/qa/v98/WP-55b/` và `WP-61a/` → `git checkout --` lại sau khi chạy.
- Bash: dùng tool Grep hoặc `rtk proxy rg`.
- Tắt mọi tiến trình nền khi xong.

### Báo cáo
`.planning/wp/v98/WP-61b-report.md`: file + symbol, test thật (lệnh + số), ảnh, bước QA iPhone, chỗ desktop còn thiếu (nếu có), rủi ro.
