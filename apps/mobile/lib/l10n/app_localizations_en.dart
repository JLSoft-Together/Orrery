// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'Orrery Remote';

  @override
  String get spikeTooltip => 'Terminal benchmark';

  @override
  String get pairMachine => 'Pair a computer';

  @override
  String get noMachines => 'No computer paired yet';

  @override
  String get noMachinesHint =>
      'Open Settings > Remote in Orrery on your computer and scan the QR code.';

  @override
  String forgetTitle(String name) {
    return 'Remove $name?';
  }

  @override
  String get forgetBody =>
      'This phone forgets the computer and its cached data. You can pair again later.';

  @override
  String get cancel => 'Cancel';

  @override
  String get forget => 'Remove';

  @override
  String lastSeenAt(String time) {
    return 'Old data, $time';
  }

  @override
  String panesCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count panes',
      one: '1 pane',
      zero: 'No panes',
    );
    return '$_temp0';
  }

  @override
  String needsYouCount(int count) {
    return '$count need you';
  }

  @override
  String runningCount(int count) {
    return '$count running';
  }

  @override
  String doneCount(int count) {
    return '$count done';
  }

  @override
  String subagentsCount(int count) {
    return '$count subagents';
  }

  @override
  String get statusWorking => 'Running';

  @override
  String get statusSubagents => 'Subagents running';

  @override
  String get statusAsked => 'Needs permission';

  @override
  String get statusFailed => 'Failed';

  @override
  String get statusDone => 'Done';

  @override
  String get statusStarting => 'Starting';

  @override
  String get statusEnded => 'Ended';

  @override
  String get statusParked => 'Parked';

  @override
  String get statusIdle => 'Idle';

  @override
  String get linkIdle => 'Not connected';

  @override
  String get linkDiscovering => 'Looking for computer';

  @override
  String get linkHandshaking => 'Connecting';

  @override
  String get linkLan => 'LAN';

  @override
  String get linkRelay => 'Relay';

  @override
  String linkReconnecting(int attempt) {
    return 'Reconnecting, try $attempt';
  }

  @override
  String get linkOffline => 'Offline';

  @override
  String get linkProtoTooOld => 'Update needed';

  @override
  String get linkRevoked => 'Revoked';

  @override
  String get reasonDesktopAbsent =>
      'The computer is not reachable through the relay.';

  @override
  String get reasonRemoteOff => 'Remote is turned off on the computer.';

  @override
  String get reasonRestarting => 'The computer is restarting Orrery.';

  @override
  String get reasonTokenExpired =>
      'Relay access expired. Join the same Wi-Fi once to renew it.';

  @override
  String get reasonRateLimited => 'Too many attempts. Waiting before retrying.';

  @override
  String get reasonNetworkLost => 'No network on this phone.';

  @override
  String get reasonNoRoute => 'No route to the computer.';

  @override
  String get reasonHandshakeFailed => 'Secure handshake failed.';

  @override
  String offlineBanner(String time) {
    return 'Computer offline, data from $time';
  }

  @override
  String get offlineNoData => 'Computer offline, no saved data';

  @override
  String get retry => 'Retry';

  @override
  String get errDenied => 'The computer refused this action.';

  @override
  String get errBusy => 'The pane is busy. Try again in a moment.';

  @override
  String get errLimit => 'Limit reached.';

  @override
  String get errNotFound => 'Not found. It may have been closed.';

  @override
  String get errInvalid => 'Invalid request.';

  @override
  String get errTimeout => 'The computer did not answer in time.';

  @override
  String get errOffline => 'Computer offline.';

  @override
  String get errRate => 'Too many commands. Slow down.';

  @override
  String get errProto => 'App and desktop versions do not match.';

  @override
  String get errInternal => 'Something went wrong on the computer.';

  @override
  String get revokedTitle => 'The computer revoked this phone';

  @override
  String get revokedBody =>
      'Saved data was deleted. Pair again from Settings > Remote on the computer.';

  @override
  String get removeMachine => 'Remove computer';

  @override
  String get protoTitle => 'Update the app or the desktop';

  @override
  String get protoBody =>
      'This app and Orrery on the computer speak different protocol versions.';

  @override
  String get back => 'Back';

  @override
  String confirmMore(int count) {
    return '$count more waiting';
  }

  @override
  String confirmExpires(int seconds) {
    return 'Cancels itself in $seconds s';
  }

  @override
  String get missionControl => 'Mission Control';

  @override
  String get inbox => 'Inbox';

  @override
  String get more => 'More';

  @override
  String reopenPane(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Reopen closed pane ($count)',
      zero: 'Reopen closed pane',
    );
    return '$_temp0';
  }

  @override
  String get openPane => 'Open pane';

  @override
  String get workspaceLoading => 'Loading workspace';

  @override
  String get workspaceEmpty => 'Workspace is empty';

  @override
  String get workspaceEmptyHint =>
      'Open a pane to start working from your phone.';

  @override
  String get otherProject => 'No project';

  @override
  String get inboxEmpty => 'Nothing needs you';

  @override
  String get inboxEmptyHint =>
      'Permission requests and finished work show up here.';

  @override
  String get inboxWaiting => 'waiting for you';

  @override
  String get inboxDone => 'finished';

  @override
  String get filterAll => 'All';

  @override
  String get filterNeeds => 'Needs you';

  @override
  String get filterRunning => 'Running';

  @override
  String get filterDone => 'Done';

  @override
  String get filterIdle => 'Idle';

  @override
  String get missionEmpty => 'No pane matches this filter';

  @override
  String get tabClosed => 'This tab was closed on the computer';

  @override
  String get paneClosed => 'The pane you were viewing was closed';

  @override
  String get tabActions => 'Tab actions';

  @override
  String get renameTab => 'Rename tab';

  @override
  String get renameHint => 'Empty = default name';

  @override
  String get save => 'Save';

  @override
  String get movePane => 'Move pane to another tab';

  @override
  String get compact => 'Compact';

  @override
  String get compactTitle => 'Compact this agent?';

  @override
  String get compactBody =>
      'Orrery sends the handoff prompt and /compact into this pane.';

  @override
  String get closePane => 'Close pane';

  @override
  String closePaneTitle(String title) {
    return 'Close $title?';
  }

  @override
  String get closePaneBody =>
      'The pane closes on the computer too. You can reopen it from the menu.';

  @override
  String get tapToView => 'Tap to view';

  @override
  String get typerYou => 'You are typing';

  @override
  String get typerViewOnly => 'View only';

  @override
  String get typeDirect => 'Type straight into the terminal';

  @override
  String get inputHint => 'Message or command';

  @override
  String get inputOffline => 'Offline, cannot type';

  @override
  String get send => 'Send';

  @override
  String get answerYes => 'Yes';

  @override
  String get answerYesAlways => 'Yes, always';

  @override
  String get answerNo => 'No';

  @override
  String get answerEsc => 'Esc';

  @override
  String get paneNotTerminal =>
      'This pane is not a terminal. Open it on the computer.';

  @override
  String get attachLimit =>
      'Only 4 terminals can be watched at once. Close one first.';

  @override
  String sessionEnded(int code) {
    return 'Session ended (code $code)';
  }

  @override
  String get sessionLost => 'Lost the terminal on the computer';

  @override
  String get pickProject => 'Project';

  @override
  String get noProjects =>
      'No project yet. Add favorites on the computer first.';

  @override
  String get pickAgent => 'Open with';

  @override
  String get shell => 'Shell';

  @override
  String get promptOptional => 'Prompt (optional)';

  @override
  String get promptHint => 'What should the agent do first?';

  @override
  String get submitNow => 'Send it right away';

  @override
  String get submitNowHint =>
      'Off: the prompt is only typed in, you press Enter later.';

  @override
  String get openPaneAction => 'Open';

  @override
  String get paneOpened => 'Pane opened';

  @override
  String get pairBadQr => 'Invalid QR code';

  @override
  String get pairExpired =>
      'QR code expired. Create a new one on the computer.';

  @override
  String get pairNoRoute => 'Cannot find the computer on this Wi-Fi.';

  @override
  String get pairNetwork => 'Connection lost while pairing. Scan again.';

  @override
  String get pairKeyMismatch =>
      'Computer key does not match the QR code. Pairing stopped.';

  @override
  String get pairTimeout => 'The computer did not answer in time.';

  @override
  String get pairDenied => 'The computer refused the pairing.';

  @override
  String get pairProtocol => 'Protocol error while pairing.';

  @override
  String get pairing => 'Pairing...';

  @override
  String get pairScanHint => 'Scan the QR code in Orrery Settings';
}
