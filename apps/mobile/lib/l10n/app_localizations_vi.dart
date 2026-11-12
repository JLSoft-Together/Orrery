// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Vietnamese (`vi`).
class AppLocalizationsVi extends AppLocalizations {
  AppLocalizationsVi([String locale = 'vi']) : super(locale);

  @override
  String get appTitle => 'Orrery Remote';

  @override
  String get spikeTooltip => 'Đo terminal';

  @override
  String get pairMachine => 'Ghép máy';

  @override
  String get noMachines => 'Chưa ghép máy nào';

  @override
  String get noMachinesHint =>
      'Mở Settings > Remote trong Orrery trên máy tính rồi quét mã QR.';

  @override
  String forgetTitle(String name) {
    return 'Gỡ $name?';
  }

  @override
  String get forgetBody =>
      'Điện thoại sẽ quên máy này và xoá dữ liệu đã lưu. Có thể ghép lại sau.';

  @override
  String get cancel => 'Huỷ';

  @override
  String get forget => 'Gỡ';

  @override
  String lastSeenAt(String time) {
    return 'Dữ liệu cũ, lúc $time';
  }

  @override
  String panesCount(int count) {
    return '$count pane';
  }

  @override
  String needsYouCount(int count) {
    return '$count cần bạn';
  }

  @override
  String runningCount(int count) {
    return '$count đang chạy';
  }

  @override
  String doneCount(int count) {
    return '$count xong';
  }

  @override
  String subagentsCount(int count) {
    return '$count subagent';
  }

  @override
  String get statusWorking => 'Đang chạy';

  @override
  String get statusSubagents => 'Subagent đang chạy';

  @override
  String get statusAsked => 'Xin quyền';

  @override
  String get statusFailed => 'Lỗi';

  @override
  String get statusDone => 'Xong việc';

  @override
  String get statusStarting => 'Đang khởi động';

  @override
  String get statusEnded => 'Đã kết thúc';

  @override
  String get statusParked => 'Tạm gác';

  @override
  String get statusIdle => 'Rảnh';

  @override
  String get linkIdle => 'Chưa nối';

  @override
  String get linkDiscovering => 'Đang tìm máy';

  @override
  String get linkHandshaking => 'Đang nối';

  @override
  String get linkLan => 'LAN';

  @override
  String get linkRelay => 'Relay';

  @override
  String linkReconnecting(int attempt) {
    return 'Nối lại, lần $attempt';
  }

  @override
  String get linkOffline => 'Offline';

  @override
  String get linkProtoTooOld => 'Cần cập nhật';

  @override
  String get linkRevoked => 'Đã thu hồi';

  @override
  String get reasonDesktopAbsent => 'Máy tính không có mặt trên relay.';

  @override
  String get reasonRemoteOff => 'Remote đang tắt trên máy tính.';

  @override
  String get reasonRestarting => 'Máy tính đang khởi động lại Orrery.';

  @override
  String get reasonTokenExpired =>
      'Quyền relay đã hết hạn. Nối cùng Wi-Fi với máy một lần để gia hạn.';

  @override
  String get reasonRateLimited => 'Thử quá nhiều lần. Đang chờ rồi thử lại.';

  @override
  String get reasonNetworkLost => 'Điện thoại không có mạng.';

  @override
  String get reasonNoRoute => 'Không có đường tới máy tính.';

  @override
  String get reasonHandshakeFailed => 'Bắt tay bảo mật thất bại.';

  @override
  String offlineBanner(String time) {
    return 'Máy offline, dữ liệu lúc $time';
  }

  @override
  String get offlineNoData => 'Máy offline, chưa có dữ liệu lưu';

  @override
  String get retry => 'Thử lại';

  @override
  String get errDenied => 'Máy tính từ chối thao tác này.';

  @override
  String get errBusy => 'Pane đang bận. Thử lại sau giây lát.';

  @override
  String get errLimit => 'Đã chạm limit.';

  @override
  String get errNotFound => 'Không tìm thấy, có thể đã đóng.';

  @override
  String get errInvalid => 'Yêu cầu không hợp lệ.';

  @override
  String get errTimeout => 'Máy tính không trả lời kịp.';

  @override
  String get errOffline => 'Máy đang offline.';

  @override
  String get errRate => 'Gửi lệnh quá nhanh. Chậm lại.';

  @override
  String get errProto => 'Phiên bản app và desktop không khớp.';

  @override
  String get errInternal => 'Máy tính gặp lỗi.';

  @override
  String get revokedTitle => 'Máy đã thu hồi điện thoại này';

  @override
  String get revokedBody =>
      'Dữ liệu đã lưu đã bị xoá. Ghép lại từ Settings > Remote trên máy tính.';

  @override
  String get removeMachine => 'Gỡ máy';

  @override
  String get protoTitle => 'Cập nhật app hoặc desktop';

  @override
  String get protoBody =>
      'App này và Orrery trên máy tính dùng phiên bản giao thức khác nhau.';

  @override
  String get back => 'Quay lại';

  @override
  String confirmMore(int count) {
    return 'Còn $count yêu cầu đang chờ';
  }

  @override
  String confirmExpires(int seconds) {
    return 'Tự huỷ sau $seconds giây';
  }

  @override
  String get missionControl => 'Mission Control';

  @override
  String get inbox => 'Inbox';

  @override
  String get more => 'Thêm';

  @override
  String reopenPane(int count) {
    return 'Mở lại pane vừa đóng ($count)';
  }

  @override
  String get openPane => 'Mở pane';

  @override
  String get workspaceLoading => 'Đang tải Workspace';

  @override
  String get workspaceEmpty => 'Workspace đang trống';

  @override
  String get workspaceEmptyHint => 'Mở một pane để làm việc từ điện thoại.';

  @override
  String get otherProject => 'Không thuộc project';

  @override
  String get inboxEmpty => 'Không có gì cần bạn';

  @override
  String get inboxEmptyHint => 'Xin quyền và việc đã xong sẽ hiện ở đây.';

  @override
  String get inboxWaiting => 'đang chờ bạn';

  @override
  String get inboxDone => 'đã xong';

  @override
  String get filterAll => 'Tất cả';

  @override
  String get filterNeeds => 'Cần bạn';

  @override
  String get filterRunning => 'Đang chạy';

  @override
  String get filterDone => 'Xong';

  @override
  String get filterIdle => 'Rảnh';

  @override
  String get missionEmpty => 'Không có pane nào khớp bộ lọc';

  @override
  String get tabClosed => 'Tab này đã đóng trên máy tính';

  @override
  String get linkUnknownMachine => 'Chưa ghép máy này';

  @override
  String get linkPaneGone => 'Pane này không còn mở';

  @override
  String get paneClosed => 'Pane đang xem đã đóng';

  @override
  String get tabActions => 'Thao tác tab';

  @override
  String get renameTab => 'Đổi tên tab';

  @override
  String get renameSession => 'Đổi tên session';

  @override
  String get renameSessionHint => 'Để trống = tên gốc';

  @override
  String get renameMachine => 'Đổi tên máy tính';

  @override
  String get renameMachineHint => 'Để trống = tên do máy tính đặt';

  @override
  String machineOriginal(String name) {
    return 'Máy tính đặt tên $name';
  }

  @override
  String get renameHint => 'Để trống = tên mặc định';

  @override
  String get save => 'Lưu';

  @override
  String get movePane => 'Chuyển pane sang tab khác';

  @override
  String get compact => 'Compact';

  @override
  String get compactTitle => 'Compact agent này?';

  @override
  String get compactBody =>
      'Orrery sẽ gửi prompt bàn giao và /compact vào pane này.';

  @override
  String get closePane => 'Đóng pane';

  @override
  String closePaneTitle(String title) {
    return 'Đóng $title?';
  }

  @override
  String get closePaneBody =>
      'Pane cũng đóng trên máy tính. Có thể mở lại từ menu.';

  @override
  String get tapToView => 'Chạm để xem';

  @override
  String get typerYou => 'Bạn đang gõ';

  @override
  String get typerViewOnly => 'Chỉ xem';

  @override
  String get typeDirect => 'Gõ thẳng vào terminal';

  @override
  String get inputHint => 'Tin nhắn hoặc lệnh';

  @override
  String get inputOffline => 'Offline, không gõ được';

  @override
  String get send => 'Gửi';

  @override
  String get answerYes => 'Có';

  @override
  String get answerYesAlways => 'Có luôn';

  @override
  String get answerNo => 'Không';

  @override
  String get answerEsc => 'Esc';

  @override
  String get paneNotTerminal =>
      'Pane này không phải terminal. Mở trên máy tính.';

  @override
  String get attachLimit =>
      'Chỉ xem được 4 terminal cùng lúc. Đóng bớt một cái.';

  @override
  String sessionEnded(int code) {
    return 'Phiên đã kết thúc (code $code)';
  }

  @override
  String get sessionLost => 'Mất terminal trên máy tính';

  @override
  String get pickProject => 'Project';

  @override
  String get noProjects =>
      'Chưa có project. Thêm favorites trên máy tính trước.';

  @override
  String get pickAgent => 'Mở bằng';

  @override
  String get shell => 'Shell';

  @override
  String get promptOptional => 'Prompt (tuỳ chọn)';

  @override
  String get promptHint => 'Agent làm gì đầu tiên?';

  @override
  String get submitNow => 'Gửi luôn';

  @override
  String get submitNowHint => 'Tắt: prompt chỉ được gõ sẵn, bạn tự Enter sau.';

  @override
  String get openPaneAction => 'Mở';

  @override
  String get paneOpened => 'Đã mở pane';

  @override
  String get pairBadQr => 'Mã QR không đúng';

  @override
  String get pairExpired => 'Mã QR đã hết hạn, tạo mã mới trên máy tính.';

  @override
  String get pairNoRoute => 'Không thấy máy trong mạng LAN.';

  @override
  String get pairNetwork => 'Mất kết nối khi ghép, quét lại mã.';

  @override
  String get pairKeyMismatch => 'Khoá máy không khớp mã QR, dừng ghép.';

  @override
  String get pairTimeout => 'Máy không trả lời kịp.';

  @override
  String get pairDenied => 'Máy tính từ chối.';

  @override
  String get pairProtocol => 'Lỗi giao thức khi ghép.';

  @override
  String get pairing => 'Đang ghép...';

  @override
  String get pushTitle => 'Thông báo';

  @override
  String get pushHint => 'Báo khi có pane cần bạn lúc app đang đóng.';

  @override
  String get pushNotSetUp =>
      'Chưa thiết lập trên máy tính. Thêm dự án Firebase trong Settings > Remote của Orrery.';

  @override
  String get pushForeign => 'Máy này dùng Firebase khác, không nhận thông báo';

  @override
  String get pushRevoked => 'Quyền thông báo đang tắt trong cài đặt hệ thống.';

  @override
  String get pushDenied =>
      'Quyền thông báo bị từ chối. Cho phép trong cài đặt hệ thống.';

  @override
  String get pushUnavailable => 'Bản này chưa dùng được thông báo.';

  @override
  String get pairScanHint => 'Quét mã QR trong Settings của Orrery';

  @override
  String get navWorkspace => 'Workspace';

  @override
  String get vMonitor => 'Quỹ đạo';

  @override
  String get vMonitorEmpty => 'Máy tính chưa có session nào.';

  @override
  String vTokens(String count) {
    return '$count token';
  }

  @override
  String vContextPct(int percent) {
    return 'context $percent%';
  }

  @override
  String get vNotInWorkspace => 'Session này chưa mở trong Workspace.';

  @override
  String get vViewerMenu => 'Explorer, Changes, Git Log, Run';

  @override
  String get vExplorer => 'Explorer';

  @override
  String get vChanges => 'Changes';

  @override
  String get vGitLog => 'Git Log';

  @override
  String get vRun => 'Run';

  @override
  String get vRetry => 'Thử lại';

  @override
  String get vErrNotRepo => 'Thư mục này không phải git repo.';

  @override
  String get vErrConflict => 'File đã đổi trên máy tính.';

  @override
  String get vErrMissing => 'File không còn tồn tại.';

  @override
  String get vErrTooLarge => 'Quá lớn để xử lý trên điện thoại.';

  @override
  String get vErrExists => 'Tên này đã tồn tại.';

  @override
  String get vErrNotMerged => 'Branch chưa được merge.';

  @override
  String get vErrCurrent => 'Đây là branch hiện tại.';

  @override
  String get vErrNoUpstream => 'Branch chưa có upstream.';

  @override
  String get vErrNonFf => 'Không fast-forward được. Xử lý trên máy tính.';

  @override
  String get vErrAuth => 'Remote từ chối thông tin đăng nhập.';

  @override
  String get vErrNothing => 'Không có gì để commit.';

  @override
  String get vErrEmptyMessage => 'Nhập commit message trước.';

  @override
  String get vErrPushed => 'Commit này đã được push.';

  @override
  String get vErrHook => 'Git hook từ chối commit.';

  @override
  String get vErrFailed => 'Lệnh thất bại.';

  @override
  String get vCancelled => 'Đã huỷ.';

  @override
  String get vWaitConfirm => 'Xác nhận trên điện thoại để tiếp tục.';

  @override
  String get vFindFile => 'Tìm file';

  @override
  String get vFindFileHint => 'Gõ một phần tên hoặc đường dẫn file.';

  @override
  String get vFindInFile => 'Tìm trong file';

  @override
  String get vNoMatches => 'Không có kết quả';

  @override
  String get vPrevMatch => 'Kết quả trước';

  @override
  String get vNextMatch => 'Kết quả sau';

  @override
  String get vListTruncated => 'Danh sách bị cắt ở giới hạn của máy tính.';

  @override
  String get vFolderEmpty => 'Thư mục trống.';

  @override
  String get vEdit => 'Sửa';

  @override
  String get vSaved => 'Đã lưu';

  @override
  String get vDiscardEditsTitle => 'Bỏ phần đã sửa?';

  @override
  String get vDiscardEditsBody => 'Phần bạn đã gõ chưa được lưu.';

  @override
  String get vKeepEditing => 'Sửa tiếp';

  @override
  String get vDiscard => 'Bỏ';

  @override
  String get vFileTooBigPhone => 'File quá lớn cho điện thoại.';

  @override
  String get vFileBinary => 'File nhị phân, không hiện được.';

  @override
  String get vFileImage => 'File ảnh, mở trên máy tính.';

  @override
  String vFileTooLarge(String size) {
    return 'File $size, quá lớn để mở ở đây.';
  }

  @override
  String get vFileNotUtf8 =>
      'File không phải text UTF-8, chỉ mở trên máy tính.';

  @override
  String get vFileChangedOnDisk =>
      'File đã đổi trên máy tính. Phần sửa của bạn chưa được lưu.';

  @override
  String get vCopied => 'Đã chép';

  @override
  String get vCopyMine => 'Chép bản của tôi';

  @override
  String get vReloadDiscard => 'Tải lại, bỏ phần sửa';

  @override
  String get vDiffBinary => 'File nhị phân, không có diff text.';

  @override
  String get vDiffTooLarge => 'Diff quá lớn để hiện.';

  @override
  String get vDiffEmpty => 'Không có khác biệt.';

  @override
  String vUnchangedLines(int count) {
    return '$count dòng không đổi';
  }

  @override
  String get vChangesClean => 'Không có thay đổi nào.';

  @override
  String vStaged(int count) {
    return 'Đã stage ($count)';
  }

  @override
  String vUnstaged(int count) {
    return 'Thay đổi ($count)';
  }

  @override
  String get vStageAll => 'Stage hết';

  @override
  String get vUnstageAll => 'Unstage hết';

  @override
  String get vStage => 'Stage';

  @override
  String get vUnstage => 'Unstage';

  @override
  String get vDiscardChanges => 'Bỏ thay đổi';

  @override
  String get vDetachedHead => 'Detached HEAD';

  @override
  String get vCommitMessage => 'Commit message';

  @override
  String get vCommit => 'Commit';

  @override
  String get vCommitted => 'Đã commit';

  @override
  String get vNoCommits => 'Chưa có commit nào.';

  @override
  String get vBranches => 'Branches';

  @override
  String get vCommitDetail => 'Commit';

  @override
  String vFilesChanged(int count) {
    return '$count file đổi';
  }

  @override
  String get vLocalBranches => 'Local';

  @override
  String get vRemoteBranches => 'Remote';

  @override
  String get vTags => 'Tags';

  @override
  String get vFetch => 'Fetch';

  @override
  String get vFetched => 'Đã fetch';

  @override
  String get vNewBranch => 'Branch mới';

  @override
  String get vBranchName => 'Tên branch';

  @override
  String vBranchFrom(String base) {
    return 'Tạo từ $base';
  }

  @override
  String get vCreate => 'Tạo';

  @override
  String vBranchCreated(String name) {
    return 'Đã tạo $name';
  }

  @override
  String get vUpdateBranch => 'Cập nhật (fast-forward)';

  @override
  String get vUpdated => 'Đã cập nhật';

  @override
  String get vDeleteBranch => 'Xoá';

  @override
  String get vUpstreamGone => 'upstream đã mất';

  @override
  String get vPushAction => 'Push...';

  @override
  String vPushTitle(String branch) {
    return 'Push $branch';
  }

  @override
  String vPushTarget(String target) {
    return 'Tới $target';
  }

  @override
  String vPushOutgoing(int count) {
    return '$count commit sẽ gửi';
  }

  @override
  String get vPushNewBranch => 'Tạo branch mới trên remote.';

  @override
  String get vAndMore => 'và nữa...';

  @override
  String get vPushed => 'Đã push';

  @override
  String get vRunNoConfigs =>
      'Project chưa có run configuration. Tạo trên máy tính.';

  @override
  String get vRunConfig => 'Configuration';

  @override
  String get vRunDevice => 'Thiết bị';

  @override
  String get vRunStart => 'Chạy';

  @override
  String get vRunHotReload => 'Hot reload';

  @override
  String get vRunRestart => 'Chạy lại';

  @override
  String get vRunStop => 'Dừng';

  @override
  String get vRunNoLog => 'Chưa có log';

  @override
  String get lockReason => 'Mở khoá Orrery Remote';

  @override
  String get lockTitle => 'Orrery Remote đang khoá';

  @override
  String get lockBody =>
      'Xác thực bằng vân tay, khuôn mặt hoặc khoá màn hình của máy để tiếp tục.';

  @override
  String get lockUnlock => 'Mở khoá';

  @override
  String get lockCancelled => 'Đã huỷ xác thực.';

  @override
  String get lockFailed => 'Xác thực lỗi, thử lại.';

  @override
  String get lockNoCredentials =>
      'Máy chưa đặt khoá màn hình. Đặt PIN, mẫu hình, mật khẩu hoặc vân tay trong cài đặt hệ thống.';

  @override
  String get lockOpenSecurity => 'Mở cài đặt bảo mật';

  @override
  String get lockTurnOff => 'Tắt khoá app';

  @override
  String get lockTurnOffTitle => 'Tắt khoá app?';

  @override
  String get lockTurnOffWarn =>
      'Bất kỳ ai cầm điện thoại này sẽ mở được session và terminal của bạn.';

  @override
  String get lockSettingsTitle => 'Khoá app';

  @override
  String get lockSwitch => 'Yêu cầu xác thực';

  @override
  String get lockSwitchHint => 'Vân tay, khuôn mặt hoặc khoá màn hình của máy.';

  @override
  String get lockAfter => 'Hỏi lại sau';

  @override
  String get lockEveryOpen => 'Mỗi lần mở app';

  @override
  String get lockMinutes15 => '15 phút';

  @override
  String get lockHour1 => '1 giờ (mặc định)';

  @override
  String get lockHours4 => '4 giờ';

  @override
  String get lockSaveFailed => 'Không lưu được cài đặt.';

  @override
  String get homeTitle => 'Máy của tôi';

  @override
  String get homeConnected => 'Đang kết nối';

  @override
  String get homeDuplicates => 'Bản trùng tên';

  @override
  String idleCount(int count) {
    return '$count rảnh';
  }

  @override
  String pairedOn(String date) {
    return 'Ghép $date';
  }

  @override
  String cleanOld(int count) {
    return 'Dọn $count bản cũ';
  }

  @override
  String get cleanOldTitle => 'Dọn bản cũ?';

  @override
  String cleanOldBody(int count) {
    return 'Điện thoại sẽ quên $count bản trùng tên với bản mới hơn. Bản mới giữ nguyên.';
  }

  @override
  String get cleanOldDone => 'Đã dọn bản cũ.';

  @override
  String get pairNewMachine => 'Ghép máy mới';

  @override
  String get enterCodeManual => 'Nhập mã thủ công';

  @override
  String get pairTabScan => 'Quét QR';

  @override
  String get pairTabCode => 'Nhập mã';

  @override
  String get pairScanTitle => 'Đưa mã QR vào khung';

  @override
  String get pairE2eTitle => 'Kết nối mã hoá đầu cuối';

  @override
  String get pairE2eBody =>
      'Điện thoại và máy dùng chung khoá Noise. Relay chỉ chuyển byte đã mã hoá.';

  @override
  String get pairCodeLabel => 'Mã ghép trên máy tính';

  @override
  String get pairCodeField => 'Mã ghép';

  @override
  String get pairCodeSubmit => 'Ghép';

  @override
  String get chipWaiting => 'Chờ bạn';

  @override
  String monFilterAll(int count) {
    return 'Tất cả $count';
  }

  @override
  String monFilterRunning(int count) {
    return 'Đang chạy $count';
  }

  @override
  String monFilterWaiting(int count) {
    return 'Chờ bạn $count';
  }

  @override
  String monFilterIdle(int count) {
    return 'Rảnh $count';
  }

  @override
  String get monFilterEmpty => 'Không có session nào trong bộ lọc này';

  @override
  String get viewModeToCompact => 'Nhóm theo trạng thái';

  @override
  String get viewModeToCards => 'Xem dạng thẻ';

  @override
  String get viewModeToGrid => 'Xem lưới mọi pane';

  @override
  String get viewModeToSingle => 'Xem từng pane';

  @override
  String get detailSubagents => 'Sub-agent';

  @override
  String detailStarted(String time) {
    return 'Từ $time';
  }

  @override
  String get detailOpenTerminal => 'Mở terminal';

  @override
  String get detailNotInWorkspace => 'Session này chưa mở trong Workspace';

  @override
  String get detailTool => 'Tool đang chạy';

  @override
  String get detailWaiting => 'Đang chờ';

  @override
  String get detailContext => 'Ngữ cảnh';

  @override
  String get detailNoSubagents => 'Chưa có sub-agent';

  @override
  String get wsTitle => 'Workspace';

  @override
  String get wsOpenFull => 'Mở toàn màn hình';

  @override
  String wsPaneHint(int count) {
    return 'Vuốt ngang để đổi pane · $count pane trong tab';
  }

  @override
  String get wsNoTabs => 'Chưa có tab nào';

  @override
  String get wsAllTabs => 'Tất cả';

  @override
  String paneSheetTitle(String tab) {
    return 'Pane trong tab $tab';
  }

  @override
  String get paneSheetOpen => 'Đổi pane';

  @override
  String diffAdded(int count) {
    return '$count thêm';
  }

  @override
  String diffRemoved(int count) {
    return '$count xoá';
  }

  @override
  String fileLines(int count) {
    return '$count dòng';
  }

  @override
  String get fileReadOnly => 'chỉ đọc';

  @override
  String get fileEditing => 'đang sửa';

  @override
  String get fileUnsaved => 'Chưa lưu';

  @override
  String get navSettings => 'Cài đặt';

  @override
  String get settingsTitle => 'Cài đặt';

  @override
  String get settingsSecurity => 'Bảo mật';

  @override
  String get settingsAppearance => 'Giao diện';

  @override
  String get settingsLanguage => 'Ngôn ngữ';

  @override
  String get settingsTheme => 'Chủ đề';

  @override
  String get settingsFollowsMachine => 'Theo máy tính';

  @override
  String get settingsAbout => 'Về app';

  @override
  String get settingsAppName => 'Orrery Remote';

  @override
  String get settingsAppHint => 'Điều khiển từ xa app Orrery trên máy tính';

  @override
  String get settingsMachine => 'Máy đang kết nối';

  @override
  String get lockShortEvery => 'Mỗi lần';

  @override
  String get lockShortMinutes15 => '15 phút';

  @override
  String get lockShortHour1 => '1 giờ';

  @override
  String get lockShortHours4 => '4 giờ';

  @override
  String get settingsPushHint =>
      'Chọn loại thông báo gửi về điện thoại này. Máy tính gửi cả khi app đang đóng.';

  @override
  String get actTitle => 'Hoạt động gần đây';

  @override
  String get actEmpty => 'Chưa có hoạt động';

  @override
  String get actStarted => 'Session bắt đầu';

  @override
  String get actEnded => 'Session kết thúc';

  @override
  String get actRenamed => 'Đổi tên';

  @override
  String get actStatus => 'Đổi trạng thái';

  @override
  String get actCleared => 'Đã xoá context';

  @override
  String get actCompacted => 'Đã compact';

  @override
  String get actAgentStarted => 'Sub-agent bắt đầu';

  @override
  String get actAgentDone => 'Sub-agent xong';

  @override
  String get actAgentFailed => 'Sub-agent lỗi';

  @override
  String get actAgentCancelled => 'Sub-agent bị huỷ';

  @override
  String get actTool => 'Tool';

  @override
  String get actToolFailed => 'Tool lỗi';

  @override
  String get actTurnStarted => 'Bắt đầu lượt';

  @override
  String get actTurnDone => 'Xong lượt';

  @override
  String get actTurnFailed => 'Lượt bị lỗi';

  @override
  String get actTurnStopped => 'Lượt bị ngắt';

  @override
  String get actSensitive => 'Thao tác nhạy cảm';

  @override
  String get pushGroupWaiting => 'Session chờ bạn';

  @override
  String get pushGroupDone => 'Session xong';

  @override
  String get pushGroupError => 'Lỗi hoặc thoát bất thường';

  @override
  String get settingsThemeSystem => 'Theo điện thoại';

  @override
  String get settingsThemeLight => 'Sáng';

  @override
  String get settingsThemeDark => 'Tối';
}
