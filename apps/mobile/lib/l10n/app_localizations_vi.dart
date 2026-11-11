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
  String get pairScanHint => 'Quét mã QR trong Settings của Orrery';
}
