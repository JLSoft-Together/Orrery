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
  String get linkUnknownMachine => 'This computer is not paired on this phone';

  @override
  String get linkPaneGone => 'That pane is no longer open';

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
  String get pushTitle => 'Notifications';

  @override
  String get pushHint =>
      'Alert me when a pane needs me while the app is closed.';

  @override
  String get pushNotSetUp =>
      'Not set up on the desktop. Add a Firebase project in Orrery Settings > Remote.';

  @override
  String get pushForeign =>
      'This computer uses a different Firebase project; notifications are off for it';

  @override
  String get pushRevoked =>
      'Notification permission is off in the system settings.';

  @override
  String get pushDenied =>
      'Notification permission was refused. Allow it in the system settings.';

  @override
  String get pushUnavailable =>
      'Notifications are not available in this build.';

  @override
  String get pairScanHint => 'Scan the QR code in Orrery Settings';

  @override
  String get navWorkspace => 'Workspace';

  @override
  String get vMonitor => 'Monitor';

  @override
  String get vMonitorEmpty => 'No sessions on the computer right now.';

  @override
  String vTokens(String count) {
    return '$count tokens';
  }

  @override
  String vContextPct(int percent) {
    return 'context $percent%';
  }

  @override
  String get vNotInWorkspace => 'This session is not open in the Workspace.';

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
  String get vRetry => 'Retry';

  @override
  String get vErrNotRepo => 'This folder is not a git repository.';

  @override
  String get vErrConflict => 'The file changed on the computer.';

  @override
  String get vErrMissing => 'The file no longer exists.';

  @override
  String get vErrTooLarge => 'Too large to handle on the phone.';

  @override
  String get vErrExists => 'That name already exists.';

  @override
  String get vErrNotMerged => 'The branch is not merged yet.';

  @override
  String get vErrCurrent => 'This is the current branch.';

  @override
  String get vErrNoUpstream => 'The branch has no upstream.';

  @override
  String get vErrNonFf => 'Cannot fast-forward. Update from the computer.';

  @override
  String get vErrAuth => 'The remote refused the credentials.';

  @override
  String get vErrNothing => 'Nothing to commit.';

  @override
  String get vErrEmptyMessage => 'Write a commit message first.';

  @override
  String get vErrPushed => 'That commit is already pushed.';

  @override
  String get vErrHook => 'A git hook rejected the commit.';

  @override
  String get vErrFailed => 'The command failed.';

  @override
  String get vCancelled => 'Cancelled.';

  @override
  String get vWaitConfirm => 'Confirm on this phone to continue.';

  @override
  String get vFindFile => 'Find file';

  @override
  String get vFindFileHint => 'Type part of a file name or path.';

  @override
  String get vFindInFile => 'Find in file';

  @override
  String get vNoMatches => 'No matches';

  @override
  String get vPrevMatch => 'Previous match';

  @override
  String get vNextMatch => 'Next match';

  @override
  String get vListTruncated => 'The list is cut at the computer limit.';

  @override
  String get vFolderEmpty => 'This folder is empty.';

  @override
  String get vEdit => 'Edit';

  @override
  String get vSaved => 'Saved';

  @override
  String get vDiscardEditsTitle => 'Discard your edits?';

  @override
  String get vDiscardEditsBody => 'The changes you typed are not saved.';

  @override
  String get vKeepEditing => 'Keep editing';

  @override
  String get vDiscard => 'Discard';

  @override
  String get vFileTooBigPhone => 'This file is too large for the phone.';

  @override
  String get vFileBinary => 'Binary file, cannot show it.';

  @override
  String get vFileImage => 'Image file, open it on the computer.';

  @override
  String vFileTooLarge(String size) {
    return 'File is $size, too large to open here.';
  }

  @override
  String get vFileNotUtf8 =>
      'This file is not UTF-8 text, so it is read-only on the computer.';

  @override
  String get vFileChangedOnDisk =>
      'This file changed on the computer. Your edits were not saved.';

  @override
  String get vCopied => 'Copied';

  @override
  String get vCopyMine => 'Copy my text';

  @override
  String get vReloadDiscard => 'Reload, drop my edits';

  @override
  String get vDiffBinary => 'Binary file, no text diff.';

  @override
  String get vDiffTooLarge => 'The diff is too large to show.';

  @override
  String get vDiffEmpty => 'No differences.';

  @override
  String vUnchangedLines(int count) {
    return '$count unchanged lines';
  }

  @override
  String get vChangesClean => 'No changes. The working tree is clean.';

  @override
  String vStaged(int count) {
    return 'Staged ($count)';
  }

  @override
  String vUnstaged(int count) {
    return 'Changes ($count)';
  }

  @override
  String get vStageAll => 'Stage all';

  @override
  String get vUnstageAll => 'Unstage all';

  @override
  String get vStage => 'Stage';

  @override
  String get vUnstage => 'Unstage';

  @override
  String get vDiscardChanges => 'Discard changes';

  @override
  String get vDetachedHead => 'Detached HEAD';

  @override
  String get vCommitMessage => 'Commit message';

  @override
  String get vCommit => 'Commit';

  @override
  String get vCommitted => 'Committed';

  @override
  String get vNoCommits => 'No commits yet.';

  @override
  String get vBranches => 'Branches';

  @override
  String get vCommitDetail => 'Commit';

  @override
  String vFilesChanged(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count files changed',
      one: '1 file changed',
    );
    return '$_temp0';
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
  String get vFetched => 'Fetched';

  @override
  String get vNewBranch => 'New branch';

  @override
  String get vBranchName => 'Branch name';

  @override
  String vBranchFrom(String base) {
    return 'Created from $base';
  }

  @override
  String get vCreate => 'Create';

  @override
  String vBranchCreated(String name) {
    return 'Created $name';
  }

  @override
  String get vUpdateBranch => 'Update (fast-forward)';

  @override
  String get vUpdated => 'Updated';

  @override
  String get vDeleteBranch => 'Delete';

  @override
  String get vUpstreamGone => 'upstream gone';

  @override
  String get vPushAction => 'Push...';

  @override
  String vPushTitle(String branch) {
    return 'Push $branch';
  }

  @override
  String vPushTarget(String target) {
    return 'To $target';
  }

  @override
  String vPushOutgoing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count commits to send',
      one: '1 commit to send',
      zero: 'Nothing new to send',
    );
    return '$_temp0';
  }

  @override
  String get vPushNewBranch => 'Creates a new branch on the remote.';

  @override
  String get vAndMore => 'and more...';

  @override
  String get vPushed => 'Pushed';

  @override
  String get vRunNoConfigs =>
      'No run configuration for this project. Create one on the computer.';

  @override
  String get vRunConfig => 'Configuration';

  @override
  String get vRunDevice => 'Device';

  @override
  String get vRunStart => 'Run';

  @override
  String get vRunHotReload => 'Hot reload';

  @override
  String get vRunRestart => 'Restart';

  @override
  String get vRunStop => 'Stop';

  @override
  String get vRunNoLog => 'No log yet';
}
