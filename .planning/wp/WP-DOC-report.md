## WP-DOC: DONE

- `CLAUDE.md`: thêm 2 câu "Since v100" (click menu, thanh nút, chip Chờ trả lời Ctrl+Alt+Q, Mission Control, toast, công tắc `clickMenu` / `menuToast`) và "Since v101" (Disk view, core-disk, MFT UAC, dọn, file trùng, organize, apps, RAM, move + junction, snapshot, Telegram, MCP disk_*, guard + dialog, cloud-only) sau câu v81; thêm "Sixth exception (v100)" ở Core principle (phím gửi sau click của chủ nhân, giống paste). Không thêm env mới (không có env runtime mới).
- `docs/claude-code-integration.md`: thêm §11as (v100: parser `termMenu.ts` từ màn live không từ hook, phím số / Up-Down / Space / Submit, Codex mouse 1003h/1006h bị chặn trên dòng menu, toast link `agent-monitor://menu?k=...`) và §11at (v101: luồng core-disk, host Electron, web; index `CCM_DATA_DIR/disk/index.bin`; MFT helper elevated + named pipe; guard canonical; không mở file cloud-only); nối liên kết Disk tools MCP ở §11w.
- `.planning/RELEASE-4.0.6.md`: thêm mục vi + en cho v100 và v101, 2 dòng "Changelog chờ". Không bump version, không đụng changelog.ts.
- Glossary check `RELEASE-4.0.6.md --all`: sạch. CLAUDE.md và docs viết tiếng Anh nên không check; không dùng em dash / "·" trong phần thêm của RELEASE.
- Lệch nhỏ: CLAUDE.md vẫn ghi target Codex 0.162.0 (WP-0c đã vô tình nâng Codex toàn cục lên 0.169.0), con không sửa câu của người khác.

### Dòng `_meta.md` đề xuất
- docs v100/v101: `CLAUDE.md` (Since v100 / v101 + sixth exception), `docs/claude-code-integration.md` §11as (click menu: `lib/termMenu.ts`, `menuToast.ts`) và §11at (Disk: `core-disk`, `diskMap/`, MFT helper).
- release notes: `.planning/RELEASE-4.0.6.md` thêm mục v100 + v101 (vi / en) và 2 dòng changelog chờ.
