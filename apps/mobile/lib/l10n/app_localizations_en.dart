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
  String get errNeedsAuth =>
      'This command needs device authentication. Update the app to send it.';

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
  String get inboxGroupMenu => 'Waiting on a menu';

  @override
  String get inboxGroupPermission => 'Asking permission';

  @override
  String get inboxGroupQuestion => 'Questions';

  @override
  String get inboxGroupClash => 'Same file';

  @override
  String get inboxGroupError => 'Errors';

  @override
  String get inboxGroupNote => 'Agent notes';

  @override
  String get inboxGroupDone => 'Done, not seen';

  @override
  String get inboxReasonMenu => 'Waiting for a menu choice';

  @override
  String get inboxReasonPermission => 'Asking permission';

  @override
  String get inboxReasonQuestion => 'Waiting for your answer';

  @override
  String inboxReasonClash(String file, String peer) {
    return 'Editing $file with $peer';
  }

  @override
  String get inboxReasonFailed => 'Failed';

  @override
  String get inboxReasonLoop => 'Failing the same way again and again';

  @override
  String get inboxReasonChecks => 'Checks failed in its folder';

  @override
  String get inboxReasonStuck => 'Quiet while working';

  @override
  String get inboxReasonNote => 'Left you a note';

  @override
  String get inboxReasonDone => 'Finished, not looked at yet';

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
  String get renameSession => 'Rename session';

  @override
  String get renameSessionHint => 'Empty = original title';

  @override
  String get renameMachine => 'Rename computer';

  @override
  String get renameMachineHint => 'Empty = name set on the computer';

  @override
  String machineOriginal(String name) {
    return 'Named $name on the computer';
  }

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
  String get pairViaLan => 'Pairing over Wi-Fi (LAN)...';

  @override
  String get pairViaRelay =>
      'Not on the same network, pairing through the relay...';

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
  String get pushContent => 'Include content';

  @override
  String get pushContentHint =>
      'Show the agent, session, project and the question in the notification. Off: fixed text only.';

  @override
  String get pushFixedDoneTitle => 'Agent finished';

  @override
  String get pushFixedDoneBody =>
      'A session finished its turn. Tap to open it.';

  @override
  String get pushFixedQuestionTitle => 'Agent has a question';

  @override
  String get pushFixedQuestionBody =>
      'A session is waiting for your answer. Tap to open it.';

  @override
  String get pushFixedPermissionTitle => 'Permission needed';

  @override
  String get pushFixedPermissionBody =>
      'A session needs your approval. Tap to open it.';

  @override
  String get pushFixedErrorTitle => 'Session error';

  @override
  String get pushFixedErrorBody =>
      'A session stopped with an error. Tap to open it.';

  @override
  String get pushFixedSecurityTitle => 'Orrery security alert';

  @override
  String get pushFixedSecurityBody =>
      'A new device, a blocked IP or a sign-in from a new place on your computer. Check Settings > Remote on the computer.';

  @override
  String get pushFixedAttentionTitle => 'Session needs attention';

  @override
  String get pushFixedAttentionBody => 'A session needs you. Tap to open it.';

  @override
  String get pushFixedRunningTitle => 'Agent is working';

  @override
  String get pushFixedRunningBody =>
      'A session has been running for a while. Tap to open it.';

  @override
  String get pushFixedOpenTitle => 'Opened from your computer';

  @override
  String get pushFixedOpenBody => 'Tap to open this pane.';

  @override
  String get pushRunningChannel => 'Running sessions';

  @override
  String get pushActionOpen => 'Open';

  @override
  String get pushMenuStale => 'The question changed, open the pane to see it';

  @override
  String get pushFixedTestTitle => 'Orrery';

  @override
  String get pushFixedTestBody =>
      'Test notification from your computer. Push works.';

  @override
  String get pairScanHint => 'Scan the QR code in Orrery Settings';

  @override
  String get navWorkspace => 'Workspace';

  @override
  String get vMonitor => 'Monitor';

  @override
  String get vMonitorEmpty => 'No sessions on the computer right now.';

  @override
  String get monLimitTitle => 'Data too large to send';

  @override
  String monLimitBody(String size, String max) {
    return 'The computer has $size of monitor data, over the $max limit per message. Close some old sessions on the computer, then Retry.';
  }

  @override
  String get monTrimmed => 'Old data was left out to fit the size limit';

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
  String get vChecklist => 'Pre-merge checklist';

  @override
  String get vClEmpty => 'No items yet. Agents and the computer add them.';

  @override
  String get vClReady => 'Ready to merge';

  @override
  String vClOpen(int count) {
    return '$count open';
  }

  @override
  String get vClByAgent => 'Added by an agent';

  @override
  String get vClCheck => 'Check';

  @override
  String get vClNone => 'No check script set';

  @override
  String get vClIdle => 'Not run yet';

  @override
  String get vClRunning => 'Running';

  @override
  String get vClPass => 'Passed';

  @override
  String get vClFail => 'Failed';

  @override
  String get vClStale => 'Out of date, new commits since the run';

  @override
  String get vClRun => 'Run check';

  @override
  String get vClRunStarted => 'Check started on the computer';

  @override
  String get vErrCheckRunning => 'A check is already running on the computer.';

  @override
  String get vErrNoScript => 'Pick a check script on the computer first.';

  @override
  String get vErrNoIntegration =>
      'Turn on shell integration on the computer first.';

  @override
  String get vErrCheckOff => 'The checklist is not available for this folder.';

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

  @override
  String get lockReason => 'Unlock Orrery Remote';

  @override
  String get lockTitle => 'Orrery Remote is locked';

  @override
  String get lockBody =>
      'Verify with fingerprint, face or your phone\'s screen lock to continue.';

  @override
  String get lockUnlock => 'Unlock';

  @override
  String get lockCancelled => 'Verification was cancelled.';

  @override
  String get lockFailed => 'Verification failed. Try again.';

  @override
  String get lockNoCredentials =>
      'This phone has no screen lock. Set a PIN, pattern, password or fingerprint in the system settings.';

  @override
  String get lockOpenSecurity => 'Open security settings';

  @override
  String get lockTurnOff => 'Turn off app lock';

  @override
  String get lockTurnOffTitle => 'Turn off app lock?';

  @override
  String get lockTurnOffWarn =>
      'Anyone who holds this phone can then open your sessions and terminals.';

  @override
  String get lockSettingsTitle => 'App lock';

  @override
  String get lockSwitch => 'Require verification';

  @override
  String get lockSwitchHint => 'Fingerprint, face or the phone screen lock.';

  @override
  String get lockAfter => 'Ask again after';

  @override
  String get lockEveryOpen => 'Every time the app opens';

  @override
  String get lockMinutes15 => '15 minutes';

  @override
  String get lockHour1 => '1 hour (default)';

  @override
  String get lockHours4 => '4 hours';

  @override
  String get lockSaveFailed => 'Could not save the setting.';

  @override
  String get homeTitle => 'My computers';

  @override
  String get homeConnected => 'Connected';

  @override
  String get homeDuplicates => 'Same name';

  @override
  String idleCount(int count) {
    return '$count idle';
  }

  @override
  String pairedOn(String date) {
    return 'Paired $date';
  }

  @override
  String cleanOld(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Remove $count old copies',
      one: 'Remove 1 old copy',
    );
    return '$_temp0';
  }

  @override
  String get cleanOldTitle => 'Remove old copies?';

  @override
  String cleanOldBody(int count) {
    return 'This phone forgets $count copies that share a name with a newer one. The newer one stays.';
  }

  @override
  String get cleanOldDone => 'Old copies removed.';

  @override
  String get pairNewMachine => 'Pair a new computer';

  @override
  String get enterCodeManual => 'Enter code manually';

  @override
  String get pairTabScan => 'Scan QR';

  @override
  String get pairTabCode => 'Enter code';

  @override
  String get pairScanTitle => 'Point the camera at the QR code';

  @override
  String get pairE2eTitle => 'End-to-end encrypted';

  @override
  String get pairE2eBody =>
      'This phone and the computer share a Noise key. The relay only carries encrypted bytes.';

  @override
  String get pairCodeLabel => 'Pairing code from the computer';

  @override
  String get pairCodeField => 'Pairing code';

  @override
  String get pairCodeSubmit => 'Pair';

  @override
  String get chipWaiting => 'Waiting for you';

  @override
  String monFilterAll(int count) {
    return 'All $count';
  }

  @override
  String monFilterRunning(int count) {
    return 'Running $count';
  }

  @override
  String monFilterWaiting(int count) {
    return 'Waiting $count';
  }

  @override
  String monFilterIdle(int count) {
    return 'Idle $count';
  }

  @override
  String get monFilterEmpty => 'No session in this filter';

  @override
  String get viewModeToCompact => 'Group by state';

  @override
  String get viewModeToCards => 'Show as cards';

  @override
  String get viewModeToGrid => 'Show all panes in a grid';

  @override
  String get viewModeToSingle => 'Show one pane at a time';

  @override
  String get detailSubagents => 'Sub-agents';

  @override
  String detailStarted(String time) {
    return 'Since $time';
  }

  @override
  String get detailOpenTerminal => 'Open terminal';

  @override
  String get detailNotInWorkspace =>
      'This session is not open in the Workspace';

  @override
  String get detailTool => 'Running tool';

  @override
  String get detailWaiting => 'Waiting for';

  @override
  String get detailContext => 'Context';

  @override
  String get detailNoSubagents => 'No sub-agents';

  @override
  String get wsTitle => 'Workspace';

  @override
  String get wsOpenFull => 'Open full screen';

  @override
  String wsPaneHint(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count panes in this tab',
      one: '1 pane in this tab',
    );
    return '$_temp0, swipe to switch';
  }

  @override
  String get wsNoTabs => 'No tab open';

  @override
  String get wsAllTabs => 'All';

  @override
  String paneSheetTitle(String tab) {
    return 'Panes in tab $tab';
  }

  @override
  String get paneSheetOpen => 'Switch pane';

  @override
  String diffAdded(int count) {
    return '$count added';
  }

  @override
  String diffRemoved(int count) {
    return '$count removed';
  }

  @override
  String fileLines(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count lines',
      one: '1 line',
    );
    return '$_temp0';
  }

  @override
  String get fileReadOnly => 'read only';

  @override
  String get fileEditing => 'editing';

  @override
  String get fileUnsaved => 'Unsaved';

  @override
  String get navSettings => 'Settings';

  @override
  String get settingsTitle => 'Settings';

  @override
  String get settingsSecurity => 'Security';

  @override
  String get settingsAppearance => 'Appearance';

  @override
  String get settingsLanguage => 'Language';

  @override
  String get settingsTheme => 'Theme';

  @override
  String get settingsFollowsMachine => 'Follows the computer';

  @override
  String get settingsAbout => 'About';

  @override
  String get settingsAppName => 'Orrery Remote';

  @override
  String get settingsAppHint => 'Remote for the Orrery app on your computer';

  @override
  String get settingsMachine => 'Connected computer';

  @override
  String get lockShortEvery => 'Always';

  @override
  String get lockShortMinutes15 => '15 min';

  @override
  String get lockShortHour1 => '1 hour';

  @override
  String get lockShortHours4 => '4 hours';

  @override
  String get settingsPushHint =>
      'Choose which events reach this phone. The computer sends them while the app is closed.';

  @override
  String get actTitle => 'Recent activity';

  @override
  String get actEmpty => 'No activity yet';

  @override
  String get actStarted => 'Session started';

  @override
  String get actEnded => 'Session ended';

  @override
  String get actRenamed => 'Renamed';

  @override
  String get actStatus => 'Status changed';

  @override
  String get actCleared => 'Context cleared';

  @override
  String get actCompacted => 'Compacted';

  @override
  String get actAgentStarted => 'Sub-agent started';

  @override
  String get actAgentDone => 'Sub-agent finished';

  @override
  String get actAgentFailed => 'Sub-agent failed';

  @override
  String get actAgentCancelled => 'Sub-agent cancelled';

  @override
  String get actTool => 'Tool';

  @override
  String get actToolFailed => 'Tool failed';

  @override
  String get actTurnStarted => 'Turn started';

  @override
  String get actTurnDone => 'Turn finished';

  @override
  String get actTurnFailed => 'Turn failed';

  @override
  String get actTurnStopped => 'Turn interrupted';

  @override
  String get actSensitive => 'Sensitive action';

  @override
  String get pushGroupWaiting => 'Session waiting for you';

  @override
  String get pushGroupDone => 'Session finished';

  @override
  String get pushGroupError => 'Error or unexpected exit';

  @override
  String get pushPauseTitle => 'Pause notifications for 1 hour?';

  @override
  String get pushPauseBody =>
      'You will not get alerts from your computers for the next hour.';

  @override
  String get pushPauseAction => 'Pause';

  @override
  String get pushPauseCancel => 'Cancel';

  @override
  String pushPausedTitle(String time) {
    return 'Notifications paused until $time';
  }

  @override
  String get pushResumeBody => 'Turn them back on now?';

  @override
  String get pushResumeAction => 'Turn on';

  @override
  String get settingsThemeSystem => 'Follows the phone';

  @override
  String get settingsThemeLight => 'Light';

  @override
  String get settingsThemeDark => 'Dark';

  @override
  String get durationLessThanMinute => '<1m';

  @override
  String durationSeconds(int n) {
    return '${n}s';
  }

  @override
  String durationMinutes(int n) {
    return '${n}m';
  }

  @override
  String durationHours(int n) {
    return '${n}h';
  }

  @override
  String durationDays(int n) {
    return '${n}d';
  }

  @override
  String get chatInbox => 'Inbox';

  @override
  String chatWaitingGroup(int n) {
    return 'Waiting for you, $n';
  }

  @override
  String get chatRecentGroup => 'Recent';

  @override
  String get chatNoSessions => 'No sessions yet.';

  @override
  String get chatNoMatch => 'No session matches.';

  @override
  String get chatSearch => 'Search';

  @override
  String get chatSearchHint => 'Search sessions';

  @override
  String get chatMoreTitle => 'More';

  @override
  String get chatNewSession => 'New session';

  @override
  String get chatNewShell => 'New shell';

  @override
  String get chatNewAdvanced => 'More options';

  @override
  String get chatWhere => 'In';

  @override
  String get chatTools => 'Files, git, run';

  @override
  String get chatPaneActions => 'Pane';

  @override
  String get chatSession => 'Session';

  @override
  String get chatPlus => 'Add';

  @override
  String get chatViewTerminal => 'View terminal';

  @override
  String get chatViewChat => 'View chat';

  @override
  String get chatOpenTerminal => 'open terminal';

  @override
  String get chatGone => 'This session is closed.';

  @override
  String get chatHello => 'Hello';

  @override
  String chatAgentAt(String agent, String project) {
    return '$agent in $project';
  }

  @override
  String chatMessage(String agent) {
    return 'Message $agent';
  }

  @override
  String chatReplyTo(String agent) {
    return 'Reply to $agent';
  }

  @override
  String get chatChipSummary => 'Sum up the project';

  @override
  String get chatChipTest => 'Run the tests';

  @override
  String get chatChipGit => 'Show git status';

  @override
  String get chatChipTypecheck => 'Fix typecheck errors';

  @override
  String get chatSend => 'Send';

  @override
  String get chatStop => 'Stop';

  @override
  String get chatStopped => 'Esc sent';

  @override
  String get chatUnsent => 'Not sent';

  @override
  String get chatResend => 'Send again';

  @override
  String get chatOlder => 'Load older';

  @override
  String get chatWorking => 'Working';

  @override
  String get chatCopy => 'Copy';

  @override
  String get chatCopied => 'Copied';

  @override
  String get chatCutMore => 'Full text in the terminal';

  @override
  String chatMore(int n) {
    return '$n more';
  }

  @override
  String chatMoreFiles(int n) {
    return '$n more files';
  }

  @override
  String chatFilesEdited(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'Edited $n files',
      one: 'Edited 1 file',
    );
    return '$_temp0';
  }

  @override
  String get chatVerbEdit => 'Edit';

  @override
  String get chatVerbRun => 'Run';

  @override
  String get chatVerbRead => 'Read';

  @override
  String get chatVerbSearch => 'Search';

  @override
  String get chatVerbWeb => 'Web';

  @override
  String get chatVerbAgent => 'Agent';

  @override
  String get chatLiveEdit => 'Editing';

  @override
  String get chatLiveRun => 'Running';

  @override
  String get chatLiveRead => 'Reading';

  @override
  String get chatLiveSearch => 'Searching';

  @override
  String get chatLiveWeb => 'Browsing';

  @override
  String get chatLiveAgent => 'Running an agent';

  @override
  String chatSumEdit(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'edited $n times',
      one: 'edited 1 time',
    );
    return '$_temp0';
  }

  @override
  String chatSumRun(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'ran $n commands',
      one: 'ran 1 command',
    );
    return '$_temp0';
  }

  @override
  String chatSumRead(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'read $n files',
      one: 'read 1 file',
    );
    return '$_temp0';
  }

  @override
  String chatSumSearch(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'searched $n times',
      one: 'searched 1 time',
    );
    return '$_temp0';
  }

  @override
  String chatSumWeb(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n web lookups',
      one: '1 web lookup',
    );
    return '$_temp0';
  }

  @override
  String chatSumAgent(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n agents',
      one: '1 agent',
    );
    return '$_temp0';
  }

  @override
  String chatSumOther(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n other tools',
      one: '1 other tool',
    );
    return '$_temp0';
  }

  @override
  String get chatSumNone => 'Tools';

  @override
  String get chatSysStop => 'Stopped';

  @override
  String get chatSysError => 'API error';

  @override
  String get chatSysLimit => 'Hit the limit';

  @override
  String get chatSysCompact => 'Compacted';

  @override
  String get chatAskPermission => 'Allow this?';

  @override
  String get chatAskChoose => 'Pick one';

  @override
  String chatPicked(String label) {
    return 'Picked $label';
  }

  @override
  String chatSent(String label) {
    return 'Sent $label';
  }

  @override
  String get chatUndo => 'Undo';

  @override
  String get chatReconnecting => 'Reconnecting';

  @override
  String chatReconnectingQueued(int n) {
    return 'Reconnecting, $n bytes not sent';
  }

  @override
  String get chatRejoined => 'Reconnected';

  @override
  String get chatOffline => 'Offline';

  @override
  String chatQueued(int n) {
    return 'Not sent yet: $n bytes';
  }

  @override
  String termUnsent(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n characters not sent',
      one: '1 character not sent',
    );
    return '$_temp0';
  }

  @override
  String get termSendUnsent => 'Send';

  @override
  String get termDropUnsent => 'Drop';

  @override
  String chatSlow(int n) {
    return 'Slow $n ms';
  }

  @override
  String get chatNoNet => 'No network';

  @override
  String chatContext(int n) {
    return 'Context $n%';
  }

  @override
  String pairConfirmTitle(String name) {
    return 'Pair with $name?';
  }

  @override
  String get pairConfirmSourceUsb => 'Via USB cable';

  @override
  String get pairConfirmSourceDevice => 'Via USB cable or an app on this phone';

  @override
  String get pairConfirmAddressUsb => 'Over USB cable';

  @override
  String get pairConfirmSourceWeb => 'Opened from a web page';

  @override
  String pairConfirmAddress(String ip) {
    return 'Address $ip';
  }

  @override
  String get pairConfirmAction => 'Pair';

  @override
  String get pairInvalidCode => 'Invalid pairing code';

  @override
  String get pairWrongNetwork =>
      'This code is not on the current Wi-Fi network';

  @override
  String get clipSendTitle => 'Send clipboard to computer';

  @override
  String get clipSendHint =>
      'Copies the text you copied on this phone into the computer clipboard, only when you tap';

  @override
  String get clipSent => 'Sent to the computer clipboard';

  @override
  String get clipEmpty => 'The phone clipboard has no text';

  @override
  String get clipTooLarge => 'The text is over 64 KB';

  @override
  String get clipSecret => 'Looks like a secret, blocked';

  @override
  String get clipDenied => 'The computer does not allow remote control';

  @override
  String get clipFailed => 'Could not send the clipboard';

  @override
  String get clipReceived => 'Copied the computer clipboard to this phone';
}
