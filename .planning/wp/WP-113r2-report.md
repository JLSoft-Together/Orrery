## WP-113r2 — DONE

### Bảng finding
| # | Finding | Quyết định | Test |
|---|---|---|---|
| V1 | pairConfirm không timeout | SỬA: `PAIR_CONFIRM_MS` 45s, hết hạn (không phải luồng USB, USB có deadline 125s riêng) -> `pairExpired` | PhoneConnectWizard.test "a pairing ticket that ends with no phone..." |
| V2 | `paired` sống qua nhiều lần chạy, kiểm lại nhầm máy cũ | SỬA: reset trong `start()`, `doTest` ưu tiên máy connected, bỏ máy revoked | "re-check ... not a revoked old one", "starting the wizard again forgets..." |
| V3 | `settle()` treo | SỬA: session biến mất sau khi đã thấy -> `session-missing`; deadline `SESSION_MAX_MS` 20 phút -> `timeout` | stores/multiRun.test.ts (2) |
| T1 | auto offer lại khi mở lại app | SỬA: `prev` undefined thì không offer; `forgetOffered` khi `pruneEnded` | apkOffer.test.ts (2) |
| T2 | devices `unmerge` hash / `loadPkg` reject / ghi đè muộn | `loadPkg`: SỬA (catch + bỏ kết quả của máy đã offline). `unmerge` hash: BỎ, UI chỉ hiện nút split cho link `chosen`, link hash là định danh serial đã xác thực | SettingsDevices.test.ts (2) |
| T3 | 2 remote cùng gợi ý 1 adb | SỬA: `suggestedAdb` | mergeDevices.test.ts |
| T4 | `pushMenu` không TTL | SỬA: TTL 2 phút (`setPushMenu` / `pushMenuStale`), ChatView bỏ pick quá hạn | chat.test.ts (pwa) |
| T5a | `parseTicket` slice `orrery.apk` | BỎ: path đã bị regex ép kết thúc `orrery.apk` ngay dòng trên | không cần |
| T5b | `runCheck` launch throw kẹt running | SỬA: bắt throw -> báo `idle` | checklist.test.ts |

### Telegram: nguyên nhân + sửa
Test "works for a pane that moved out of the project folder" fail không ổn định (`{ok:false,error:'gone'}`), tuỳ tải máy, các test khác cũng fail xen kẽ. Debug: store của test sau chứa 3 pane cùng `sessionId` `pty-a` (2 pane trạng thái `starting` từ test trước). Nguyên nhân: persist timer 400ms của workspace store (test trước) ghi localStorage sau `localStorage.clear()`, store mới restore pane cũ, `workspace.panes.find(sessionId)` trúng pane `starting` nên trả `gone`. Code đúng, lỗi cô lập test. Sửa test: `vi.useFakeTimers({ toFake: ['setTimeout','clearTimeout'] })` trong `beforeEach`, `useRealTimers` trong `afterEach`. Chạy 3 lần liên tiếp xanh (trước sửa fail 2 trong 3 lần).

### Audit
Diff gồm: `remote/components/connect/useConnectWizard.ts` (+test), `remote/lib/connectFlow.ts`, `run/stores/{multiRun,run}.ts`, `run/lib/apkOffer.ts`, `devices/composables/useSettingsDevices.ts`, `devices/lib/mergeDevices.ts`, `git/lib/checklist.ts`, `apps/pwa/src/{lib/deepLink.ts,stores/push.ts,views/ChatView.vue}`, test tương ứng. Không comment mới, không key i18n mới.

### Test
- `npm test -w @ccm/web -- src/features/remote src/features/run src/features/devices src/features/git src/features/telegram src/features/workspace --hookTimeout=60000` -> 148 file, 1257 test xanh.
- PWA vitest -> 33 file, 311 test xanh.
- `npm run typecheck` trong `apps/web` -> sạch (đã sửa 1 lỗi kiểu trong test mới); phần pwa/shared không đổi kiểu, vitest pwa xanh.
- Test mới viết cùng lượt với fix; riêng multiRun (SESSION_MAX_MS chưa tồn tại) và test Telegram đã thấy fail trước khi sửa. Các test khác CHƯA chạy ở trạng thái "fail trước" riêng (bị resource-guard chặn lúc đó).

### Hot file cần thêm
Không.

### Commit message đề xuất
fix(remote-ui): expire stuck pair confirm, reset paired phone on wizard start, bound multi-run waits, ttl for push menu picks, stabilize telegram answer test
