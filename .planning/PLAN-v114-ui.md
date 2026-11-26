# PLAN v114: UI/UX đợt G (G1-G5)

Nguồn: `.planning/IDEAS-2026-11-26.md` mục "Đợt G", wireframe https://orrery.jinvu.vn/new_feat_2611/#gop-y. Spec: `.planning/SPEC-v114-ui.md`. Chủ nhân duyệt làm cả 5 (27/11).

## Mục tiêu
Giảm số bước và số lần đổi màn cho 5 việc hay làm: trả lời các session đang chờ (G1), gõ lại prompt quen (G2), chỉnh độ dày giao diện (G3), xem session khác đang làm gì mà không đổi tab (G4), biết cái gì mới sau khi đi vắng trên điện thoại (G5).

## Nguyên tắc giữ
- Core principle: không tự gửi prompt, không tự trả lời menu, không gọi LLM. G1 trả lời menu chỉ khi chủ nhân bấm (ngoại lệ 6, 7 sẵn có). G2 chỉ chèn chữ, chủ nhân bấm Enter / Gửi. G5 suy từ event, không LLM. Không thêm ngoại lệ mới.
- Workspace mượt là ưu tiên số 1 (`ide-performance-first`): G4 không vẽ gì khi không rê; G3 chỉ đổi CSS token.
- UI next gen (`ui-next-only`): làm cho `:root[data-ui='next']`, mist / rail; PWA chỉ theo nếu rẻ.
- Mọi chuỗi en + vi, màu qua token, trạng thái icon + chữ + màu, motion 220 / 160ms, reduced-motion.

## Phase / WP (sau review + quyết định D1-D11 trong SPEC)
| WP | Nội dung | Phạm vi chính | Lượt | Model |
|---|---|---|---|---|
| WP-114a | G4 peek rail | `features/next/components/rail/*`, `next/lib/railPeek.ts` | 1 | sonnet |
| WP-114c | G1 desktop: `ws.snooze`, `ws.reply`, railSnooze bền + since, summary `question` + `snoozed`, push bỏ pane đang hoãn | web `features/remote`, `click-menu`, `next/lib/railSnooze.ts`, `workspace` (prefillWaiting), shell `phone_ws.rs`, `push_watch.rs` | 1 | sonnet |
| WP-114f | G2 desktop: core-desk `prompts.json` + kênh, feature `prompts`, palette `;`, Settings, header pane, lưu từ tìm prompt | core-desk, `features/prompts`, `features/settings`, palette, `paneHeadActions` | 1 | sonnet |
| WP-114b | G3 mật độ | `features/next/density.css`, `RailRowItem.vue` class, Settings Mist Appearance | 2 | sonnet |
| WP-114d | G1 phone: hàng đợi từng thẻ, nút Còn N, deep link queue | `apps/mobile/lib/ui/queue/*`, inbox, pane_link | 2 | sonnet |
| WP-114g | G2 phone + shell op `prompts.*` | shell `viewer_map.rs` / file mới, `apps/mobile` composer chip | 2 | sonnet |
| WP-114e | G5 phone đi vắng | `apps/mobile/lib/remote/away_summary.dart`, màn machine | 3 | sonnet |

Sau lượt 3: review độc lập (Rust opus, TS sonnet, Flutter sonnet) → sửa → test tổng (vitest, typecheck, cargo shell + core, flutter, E2E desktop) → merge `master` → release 4.0.11.

## Release 4.0.11 (chủ nhân yêu cầu 27/11)
- Bump `apps/shell/package.json` 4.0.11, `apps/mobile/pubspec.yaml` 4.0.11+1, `changelog.ts`, `.planning/RELEASE-4.0.11.md` (v113 + v114).
- APK: build release arm64 ở máy này (keystore `D:\keys\orrery`), upload asset `orrery-remote.apk` vào release draft `v4.0.11` trước khi push tag; `desktop.yml` thêm bước tải APK từ release (`gh release download`) và đặt `ORRERY_APK`; bước tạo release đổi sang cập nhật release có sẵn.
- Merge `feat/new_feat_2611` vào `master` (merge commit, không squash), push, tag `v4.0.11`.

## Hot file (1 chủ = main)
`apps/web/src/i18n/{en,vi}.ts` + snapshot, `apps/shell/src/remote/{mod,ctx}.rs` (kênh, caps), `apps/shell/shim/preload.ts` + `shim.js`, `apps/web/src/core/lib/desktop.ts`, `apps/mobile/pubspec.yaml`, `packages/shared/src/types.ts`, `.code_index/*`, `docs/*`.

## Commit
Như v113: từng WP nghiệm thu commit đúng file, push cuối mỗi lượt khi test tổng pass, không trailer.

## Done
- 5 tính năng chạy được trên desktop dev (`npm run shell:dev`) và A33; E2E mới xanh; vitest, cargo, flutter xanh; ảnh từng state ở `docs/qa/v114/`.
- `.code_index`, `docs/remote-integration.md` (op mới), `QA-mac-checklist.md` cập nhật.
