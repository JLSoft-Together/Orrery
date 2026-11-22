# AUDIT v110 P0: `apps/web/src` chia theo feature

Gói P0 của `.planning/PLAN-v110-modules.md`, 2026-11-22, chỉ đọc. Số liệu từ script dựng đồ thị import (1 210 file `.ts` / `.vue`, chỉ import tương đối vì web không có alias `@/`), gán file vào feature bằng luật theo đường dẫn + tên file. Test đi theo file nguồn. "out" = số cạnh import sang feature khác (không tính core), "in" = feature khác import vào.

## 1. Cây đề xuất

```
apps/web/src/
  main.ts  App.vue  style.css  themes.css  fonts.css  env.d.ts
  core/                 dùng chung, không import feature nào (trừ qua registry ở mục 4)
    ws/                 stores/connection, terminals, agents, activity, extras, toasts, navigation
    desktop/            lib/desktop.ts, lib/preview.ts (client IPC files / git), lib/platform.ts
    theme/              lib/themes, fonts, appearance, motion; components/ui/ThemePicker, FontPicker
    settings/           stores/settings.ts, stores/ui.ts, stores/uiGen.ts + lib/uiGeneration.ts
    i18n/               index.ts, en.ts, vi.ts (chỉ gộp i18n của feature)
    ui/                 components/ui/*, AppSelect, ToggleSwitch, KeyHint, Pager, PanelHeader, PanelSplitter,
                        StatusBadge, StatusIcon, CountNum, FlipChar, ArtImage, LottieArt, ToastHost, ZoomHud, InfoHint
    lib/                format, fuzzy, dialog*, menu, status, provider, title, sessionGroups, codeView, projects
    stores/             favorites, peers, toolSpans (hub: 12 / 8 / 5 feature cùng đọc)
  features/<feature>/
    index.ts            API công khai duy nhất cho feature khác
    README.md           đường vào, file chính (≤ 30 dòng)
    components/ lib/ stores/ composables/ i18n/
```

Luật: feature A chỉ import `core/**` hoặc `features/B/index.ts`; không import sâu vào file của B. Kiểm bằng một test vitest nhỏ đọc đồ thị import (giống script P0).

## 2. Feature (23 + core)

| Feature | File (+test) | Dòng | Nguồn hiện tại (glob / nhóm) | out chính | in chính | File > 800 dòng |
|---|---|---|---|---|---|---|
| core | 92 (+36) | 11 453 | xem cây mục 1; `components/AppHeader.vue`, `HeaderStatusPopover.vue`, `ShortcutsDialog.vue`, `composables/{useAppToasts,useBreakReminder,useMcpCalls,useNotifications,useShortcuts,useDismiss,usePageActive,useFullscreen,useInlineEdit,useCountUp,useBump}` | (core không được out; hiện 13 file core import feature, mục 4) | mọi feature | `i18n/en.ts` 1 302, `vi.ts` 1 305 |
| workspace | 102 (+36) | 16 931 | `components/workspace/{TermPane,WorkspaceView,Pane*,Split*,Tab*,Layout(s)Dialog,LaunchPicker,DropTerminal,TermFind,ViewPaneControls,WorkspaceFooter,BrowserPane,BrowserToolbar,…}`, `components/workspace/view/*`, `lib/{term*,pane*,split*,layout(.|Presets|Snap|Undo|SplitHint|PaneMenu),tab*,workspace*,shell*,ptyClient,webglLru,dust,windowCopy,launcher,layouts,projectSwitcher,focusMode,rowWindow}`, `stores/{workspace,attention,awake}`, `composables/{usePane*,useShellMarks,useWindowCopy,useWorkspaceLinks,useRevealInWorkspace,useOpenProject,useProject*,useWheelZoom,useRunMarks,useFocusMode}`, `i18n/{workspace-v25..v77,workspace-tier2,pane-v9*,tabs-v93,split-v97,launcher-v93,shellmarks-v73,projects-v78}` | agent-tools 30, rail 20, monitor 18, next 17, files 16, run 13 | 19 feature; `stores/workspace.ts` là hub số 1 | `WorkspaceView.vue` 2 371, `stores/workspace.ts` 1 999, `TermPane.vue` 1 654 |
| files (Explorer + editor) | 51 (+21) | 11 439 | `lib/explorer/*`, `lib/editor/*`, `components/workspace/{Explorer*,FilePane,CodeEditor,CodeMinimap,FileTypeIcon,CreateEntryDialog,QuickOpen,OutlinePicker,NavPicker,ProblemBar,HttpPanel,JsonTree,AccessPrompt}`, `lib/{fileEditor,highlight*,markdown*,jsonTree,searchView,todoView,checks,problems,fileLinks,autoSave,httpFile,navigation}`, `stores/{explorer,checks}`, `i18n/{workspace-explorer,workspace-viewer,file-editor-v42,explorer-v93}` | git 22, run 13, workspace 9, disk 7 | workspace 16, git 13 | `lib/explorer/fileIconRules.ts` 2 043 (sinh bởi `scripts/gen-file-icons.mjs`, giữ nguyên), `FilePane.vue` 1 899 |
| git (diff, log, changes, commit, worktree) | 57 (+14) | 10 792 | `components/workspace/log/*`, `components/workspace/{ChangesPane,DiffView,SplitDiff,CommitBox,ConflictView,HunkPopup,DiscardConfirm,LocalHistoryDialog,Worktree*,ImageCompare,TextCompare*,ChangeRuler,CheckpointsPanel,LogFileView}`, `lib/{diff*,splitDiff,changes*,commitGraph,gitRefs,blame,logView,logQuery,mergeConflicts,patch,imageCompare,conflicts}`, `stores/checkpoints`, `composables/{useGitLog,useFileChanges,useExplorerChanges,useTurnChanges,useTurnCheckpoints,useDiffDecor}`, `i18n/workspace-{diff,log,commit}` | workspace 16, files 13, agent-tools 7 | files 22, agent-tools 10 | `ChangesPane.vue` 1 170, `i18n/workspace-commit.ts` 1 372 |
| run (Run pane, Layout / Flutter preview, strings, release, libs) | 47 (+15) | 10 404 | `components/workspace/{Run*,LayoutPreview,LayoutBoxes,FlutterPreviewPanel,DrawablePreview,StringsDialog,ReleaseCheckDialog,LibUpdatesDialog,EnvCheckDialog,I18nScanDialog,ScriptsDialog}`, `components/{RunSdkSettings,XmlPreviewSettings}.vue`, `lib/{run*,mcpRun,layoutPreview,flutterPreview,drawablePreview,releaseCheck,libVersions,xmlPreviewInstall,memoryDoctor,i18nReport}`, `lib/strings/*`, `stores/run`, `i18n/{run-v31,run-v72,layout-v32,flutter-preview-v40,strings-v74,release-v75,libs-v74,libs-v75,xml-preview-v88,memdoc-v75}` | workspace 9, monitor 6 | core 26, files 13, workspace 13 | `LayoutPreview.vue` 988 |
| agent-tools (prompt, attachments, turn review, clash, handoff / compact, plan board) | 60 (+28) | 7 551 | `lib/{prompt*,agentHandoff,compactHandoff,turn*,review*,clash,overlap,agentTouches,whoIsEditing,permAdvisor,resumeSchedule,planBoard,noteLinks,ctxSources,contextWarn,spawnQueue,sessionTools}`, `stores/{prompts,promptQueue,attachments,agentTouches,turnEdits,overlap,resumeSchedule}`, `composables/{useCompact,usePaneAttachments,useSessionTools,useClashActions,useBugReportPrompts}`, `components/workspace/{PromptPicker,PromptQueueButton,AttachmentsPanel,AttachmentLook,TurnCard,TurnReview,ClashChip,OverlapChip,HandoffMenu,PermAdvisorDialog,PlanBoard,NotesPanel,ContextMeter,ContextSourcesDialog,LimitResumeChip,SessionToolsSheet,SessionFilterBar}` | workspace 20, git 10 | workspace 30, core 23 | |
| next (UI next gen v102) | 53 (+22) | 7 087 | `components/next/**`, `lib/next/*`, `next/*`, `i18n/{mist-v102,mist-remote,rail-select-v98,v105-*,v106-rail}` | workspace 25, rail 15 | settings 16, workspace 17 | `LiveRail.vue` 829 |
| disk | 57 (+23) | 8 463 | `components/disk/*`, `components/{DiskBadge,DiskSettings,BuildCleanDialog}.vue`, `components/settings/SettingsDisk*.vue`, `lib/{disk*,organize,treemap,sunburst,buildClean}`, `stores/{disk*,organize}`, `composables/useDiskSizes`, `i18n/disk-*` | workspace 6 | core 22, settings 7, files 7 | |
| map (Agent Map, Quỹ đạo, board) | 31 (+10) | 5 838 | `components/{AgentMap,MainNode,PeerEdge,TreeEdge,SubPill}.vue`, `components/map/*` (trừ `useOfficeControls`), `components/monitor/{Orbit*,Board*,SessionBoard}.vue`, `lib/{map*,orbit*,starMap,stableEdges,boardV95}`, `composables/{useMapNode,useBoardLastTurn}`, `i18n/{map-v47,orbit-v95,peer-*,board-v95}` | monitor 29, workspace 27, office 5 | core 13, monitor 9 | `AgentMap.vue` 1 581 |
| monitor (session card, compact, jobs, sensitive) | 46 (+12) + 6 sensitive | 4 598 + 663 | `components/{SessionCard*,SessionTabs,MonitorView,FavoritesList,JobsList,CompactView,WaitingStrip,DoneTray,Subagent*,Context{Gauge,Ring,Tokens},TokenCount,ProjectStrip,ProviderBadge,AgentDetails,Sensitive*}.vue`, `components/monitor/{CommandPanel,ModeBar,OpenPaneButton}.vue`, `lib/{monitorRows,sessionSort,agentStatus,agentTools,cardMenu,subagent*,waitingStrip,sensitive}`, `stores/{agentStatus,snooze,sensitive}`, `composables/{useMonitor*,useAgentActions,useSessionActions,useSessionMenu}`, `i18n/{session-v53,monitor-v53,ui-v79-monitor,ui-v88-monitor,agent-status,sensitive-v76}` | workspace 20, map 9 | map 29, workspace 18 | |
| settings (trang Settings, Quick setup, tour) | 50 (+6) | 4 936 | `components/settings/*` (trừ `remote/`, `SettingsDisk*`), `components/{SettingsMenu,QuickSetup,OnboardingTour,BackupSettings,CaptureSettings,GitHubAccountsSettings,AgentDataSettings,AwakeSettings,KeepAwakeBadge}.vue`, `lib/{tour,onboardingGate,backup,capture,github,agentData,bugReport,errorLog,settingsFlash}`, `composables/useSettingsEnv`, `i18n/{theme-v95,theme-v96,ui-v79-settings,backup-v74,capture-v76,github-v58,agentdata-v76,bug-report-v90}` | next 16, disk 7, update 7 | core 23 | |
| remote (điện thoại, relay, push) | 33 (+18) | 3 954 | `components/settings/remote/*`, `components/{PhoneSettings}.vue`, `components/workspace/RemotePhoneChip.vue`, `lib/{remote*,phone*}`, `stores/remote`, `composables/useRemote*` (9), `i18n/{remote-*,v106-install}` | workspace 7, click-menu 6 | core 10 | |
| devices | 30 (+8) | 5 488 | `components/devices/*`, `components/workspace/{DeviceMenu,ApkSizeDialog}.vue`, `lib/{device*,devices,emulators,apkSize,appData,browserDevices}`, `stores/{devices,emulators}`, `composables/useDeviceHandoff`, `i18n/{devices-v*,emulators-v74,apk-v73,appdata-v73}` | resources 4 | core 16 | |
| rail (rail classic, sidebar) | 28 (+9) | 3 920 | `components/workspace/rail/*`, `components/workspace/WorkspaceSidebar.vue`, `lib/rail/*`, `lib/{sidebarRows,sideWidth,projectGroups}`, `composables/{useRail,useRailDrag,useClusterDrag}`, `i18n/{workspace-rail,workspace-v50-rail,workspace-v52-rail,rail-v92,groups-v99}` | workspace 29 | workspace 20, next 15 | `WorkspaceSidebar.vue` 1 031 |
| telegram | 9 (+8) | 3 368 | `components/{TelegramSettings,TelegramToggle,TelegramAccountScan}.vue`, `components/workspace/TelegramLinkDialog.vue`, `lib/telegram`, `stores/telegram*`, `composables/useTelegram*` | workspace 6 | workspace 7 | `TelegramSettings.vue` 1 556 |
| click-menu (v100 menu, inbox) | 16 (+12) | 1 660 | `components/inbox/*`, `components/workspace/TermMenuBar.vue`, `lib/{termMenu*,inbox,menuToast}`, `stores/{inbox,menuQueue}`, `composables/{useTermMenu,useTermMenuMouse,useMenuToast,useInboxGlobal}`, `i18n/click-menu-v100`, `lib/__fixtures__/term-menu` | workspace 9 | remote 6 | |
| usage | 14 (+2) | 2 493 | `components/usage/*`, `components/PlanLimits.vue`, `stores/usage`, `i18n/{usage-v53,ui-v80-limits}` | (0) | core 8 | |
| history | 14 (+5) | 1 806 | `components/history/*`, `components/{AwaySummary,RecapDialog}.vue`, `lib/{heatmap,toolStats,toolTimeline,recap}`, `stores/{history,promptSearch,away}`, `i18n/{toolstats-v73,promptsearch-v75}` | workspace 5 | core 6 | |
| resources (Resources, DevTools, diagnostics) | 13 (+4) | 1 987 | `components/{ResourcesDialog,ResourcesButton,DevToolsDialog,DiagnosticsDialog,PortConflictBar}.vue`, `lib/{resources,devTools,devPerf,devServer,devFiles}`, `stores/resources`, `composables/useDiagnosticsReport`, `i18n/{resources-v76,devtools-v75}` | workspace 2 | core 11, devices 4 | |
| extensions | 13 (+2) | 2 050 | `components/extensions/*`, `lib/{extensions,claudeHooks}`, `stores/extensions`, `i18n/{extensions-v28,hooks-v74}` | workspace 2 | core 5 | |
| update (update, What's new, tips) | 10 (+3) | 1 342 | `components/{UpdateDialog,UpdateSettings,WhatsNewDialog}.vue`, `lib/{update,changelog,tips}`, `stores/{update,tips}`, `i18n/{update-v74,whats-new-v71}` | settings 2 | settings 7, workspace 5 | |
| palette (Ctrl+K) | 4 (+2) | 1 203 | `components/CommandPalette.vue`, `lib/{palette*,paletteKey,paletteSearch}`, `i18n/ui-v80-palette` | 14 feature | core 3 | `CommandPalette.vue` 1 042 |
| office | 40 (+30) | 7 770 + `packages/habblaud` 43 694 | `office/**`, `components/office/*`, `components/map/useOfficeControls.ts`, `stores/office*`, `i18n/office-v6*` | workspace 1 | map 5, core 1, settings 1 | `office/iso/IsoRenderer.ts` 2 510 |
| deepseek (có sẵn) | 5 (+2) | 723 | `features/deepseek/*` (đã đúng mẫu) | settings 1 | workspace 2 | |

## 3. Hub nên đưa về core (feature khác đọc nhiều)

`stores/workspace.ts` (19 feature) ở lại `features/workspace` nhưng chỉ lộ qua `index.ts`. Đưa về core: `lib/title.ts` (11), `stores/favorites.ts` (12), `stores/uiGen.ts` (9), `lib/sessionGroups.ts`, `stores/peers.ts`, `stores/toolSpans.ts`, `lib/codeView.ts` (git dùng 7), `composables/useRevealInWorkspace.ts` (5 feature; đổi thành hàm trong `core/workspaceBridge` gọi qua event bus hoặc giữ ở `features/workspace/index.ts`).

Vòng phụ thuộc phải chuyển cùng đợt: workspace ↔ rail (29 / 20), workspace ↔ next (17 / 25), next → rail 15, files ↔ git (22 / 13), map → monitor 29 và monitor → map 9.

## 4. Core đang import feature (13 file, phải gỡ trước hoặc giữ ở tầng app)

- `App.vue` (44 import feature), `components/AppHeader.vue` (11), `HeaderStatusPopover.vue` (9): là tầng app, để ngoài `core/` (giữ ở `src/` gốc hoặc `src/app/`).
- `stores/connection.ts`: import `office/feed.ts`, `stores/{activity,agents,extras,favorites,…}`, `lib/patch.ts` (git). Gỡ: connection phát event qua một registry `onEvent(type, handler)`; office / git đăng ký. Đây là điểm nối Office chính.
- `stores/ui.ts`: import `lib/tour`, `components/settings/index`, `lib/devTools`, `lib/layout`, `lib/devices`, `lib/diskMap`.
- `composables/useNotifications.ts` (17), `useMcpCalls.ts` (8), `useAppToasts.ts` (4): chuyển sang tầng app hoặc tách handler về từng feature.
- `i18n/en.ts`, `vi.ts`: import 103 file i18n theo version (`<feature>-v<n>.ts`, mỗi file `export default { en, vi }`, gộp sâu). Đề xuất: mỗi feature có `i18n/index.ts` gộp file của nó, `core/i18n/en.ts` chỉ import 23 barrel. Giữ nguyên thứ tự gộp để không đổi key trùng (kiểm bằng snapshot `JSON.stringify(messages)` trước / sau).
- `lib/desktop.ts` → `lib/{preview,layoutPreview,drawablePreview}`, `lib/mcpRun.ts` → run, `lib/refPrefs.ts` → git, `lib/footerStatus.ts` → rail, `lib/errorLog.ts` → settings, `KeyHint.vue` / `LottieArt.vue` / `ShortcutsDialog.vue` / `FontPicker.vue` / `ThemePicker.vue` → `stores/settings` (sẽ là core, hết vấn đề), `StatusBadge` / `StatusIcon` → `lib/status` (core), `ZoomHud.vue` → `useWheelZoom` (workspace).

## 5. Thứ tự chuyển (mỗi đợt 1-2 feature, ít phụ thuộc trước)

| Đợt | Feature | Lý do |
|---|---|---|
| 0 | Chuẩn bị: alias `@core` / `@features` trong `vite.config.ts` + `tsconfig.json` + vitest; codemod đổi import (ts-morph hoặc script, chỉ in số chỗ); test luật import; snapshot i18n | Không dời file, không đổi hành vi |
| 1 | office (= P2), usage | office: in 7, đã lazy; usage: out 0 |
| 2 | extensions, update | out ≤ 3 |
| 3 | history, resources | out ≤ 5 |
| 4 | devices, disk | out ≤ 6, phụ thuộc vào `lib/diskMap` từ core cần gỡ cùng đợt |
| 5 | telegram, remote | remote → click-menu 6 (click-menu chuyển sau vẫn được vì import qua path cũ) |
| 6 | click-menu, palette | palette import 14 feature, chuyển khi đa số đã có `index.ts` |
| 7 | settings, run | |
| 8 | files + git | vòng 22 / 13 |
| 9 | agent-tools | workspace import 30 |
| 10 | monitor (+ sensitive) + map | vòng 29 / 9 |
| 11 | workspace + rail + next | vòng lớn nhất, 3 file > 1 600 dòng |
| 12 | phần còn lại → `src/core`, tầng app `src/app`, gỡ 13 file core import feature | |

Mỗi đợt: `git mv`, codemod import, `npm run qa` + E2E map của feature, `npm run typecheck -w @ccm/web`.

## 6. Rủi ro

- **Import tương đối, không alias**: dời 1 file phải sửa mọi đường `../`; 52 `vi.mock('../…')` trong 39 test cũng mang đường dẫn. Làm đợt 0 trước (alias + codemod) rồi mới dời.
- **`import.meta.glob` theo vị trí file**: `components/ArtImage.vue:19` (`../assets/art/*.svg`), `components/LottieArt.vue:11` (`../assets/art/*.json`). Dời 2 file này phải đổi glob.
- **Test đọc source theo đường dẫn**: `components/office/officeView.test.ts` (`../AgentMap.vue?raw`, `../map/useOfficeControls.ts?raw`), `next/palettes.test.ts` (`../style.css`, `../themes.css`, `../../index.html`), 12 test dùng `readFileSync` / `import.meta.url`, 7 file dùng `lib/__fixtures__`.
- **E2E đọc file web**: `apps/desktop/e2e/v100-menu-bar.cjs:8` (`apps/web/src/lib/__fixtures__/term-menu`), `v101-diskclean.cjs:91` (`apps/web/src/i18n/disk-clean-v101.ts` qua `fs`). Selector `data-*` không đổi.
- **`scripts/qa-fast.mjs` `E2E_MAP`** khớp theo đoạn đường dẫn: `[\\/]explorer[\\/]`, `[\\/]git[\\/]`, `stores[\\/]toasts`, `extensions[\\/]`, `features[\\/]deepseek[\\/]`, `[\\/]layout[\\/]`. Sau khi có `features/git/`, `features/extensions/` thì mọi file trong đó kéo E2E `log` / `v89-*`; phải viết lại map theo feature.
- **`vite.config.ts`**: `chunkFileNames` dựa vào `/features/deepseek/`; alias `./locales/all` của habblaud; đọc version từ `../desktop/package.json` (đổi ở P1).
- **Script sinh file**: `scripts/gen-file-icons.mjs` ghi `lib/explorer/fileIconRules.ts`, `scripts/gen-fonts.mjs` ghi `fonts.css`; đổi đường ra khi dời.
- **i18n gộp**: key trùng giữa file version (gộp sâu theo thứ tự import) có thể đổi kết quả nếu đổi thứ tự; cần snapshot.
- **Pinia store id** không đổi theo đường dẫn, an toàn; `localStorage` key không liên quan.
- Gói v109 đang sửa `themes.ts`, `uiGen.ts`, `WorkspaceSidebar.vue`, `LiveRail.vue`, `useClusterDrag.ts`, `themes.css` (git status): đợt 11 / 12 chỉ chạy khi v109 xong.

## 7. Office: tách module lazy (P2)

Hiện tại:
- Đã lazy: `components/AgentMap.vue:65` `defineAsyncComponent(() => import('./office/OfficeView.vue'))`; world Habblaud nạp từ OfficeView.
- Còn nạp tĩnh vào bundle chính: `AgentMap.vue:55-56` (`OfficeOptions.vue`, `OfficeNeedsNode.vue`), `stores/office.ts` (import `office/sound`, `office/packs`, `office/iso/index.ts`; `iso/index.ts` chỉ import type từ `@ccm/habblaud/client/src/world` nên nhẹ), `stores/connection.ts:5` (`office/feed.ts`, import type `@ccm/habblaud/shared/types` + `localize.ts`), `components/settings/SettingsGeneral.vue:5` (`OfficeNeedsNode.vue`), `components/map/useOfficeControls.ts`, `i18n/en.ts` + `vi.ts` (3 file `office-v60/62/63`).
- Đề xuất: `features/office/` gom `office/**`, `components/office/*`, `useOfficeControls`, `stores/office*`, i18n office. `index.ts` chỉ lộ `OfficeEntry` (async component), `officeEnabled()`, `registerOfficeFeed(connection)`. Connection gọi registry thay vì import `officeFeed`; feed chỉ đăng ký khi Office bật. i18n Office nạp bằng `i18n.global.mergeLocaleMessage` lúc mở Office (đổi nhỏ: thêm 1 lần chờ). `OfficeNeedsNode.vue` thay bằng 1 dòng chữ trong settings (Office chỉ chạy với collector Node, sẽ mất ở P1).
- Index riêng: `.code_index/office.md` (đọc chỉ khi làm Office), `packages/habblaud` ghi chú ở đó, không vào `web.md`.
