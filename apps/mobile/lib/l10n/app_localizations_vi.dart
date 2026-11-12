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
}
