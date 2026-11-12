import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_vi.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('vi'),
  ];

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'Orrery Remote'**
  String get appTitle;

  /// No description provided for @spikeTooltip.
  ///
  /// In en, this message translates to:
  /// **'Terminal benchmark'**
  String get spikeTooltip;

  /// No description provided for @pairMachine.
  ///
  /// In en, this message translates to:
  /// **'Pair a computer'**
  String get pairMachine;

  /// No description provided for @noMachines.
  ///
  /// In en, this message translates to:
  /// **'No computer paired yet'**
  String get noMachines;

  /// No description provided for @noMachinesHint.
  ///
  /// In en, this message translates to:
  /// **'Open Settings > Remote in Orrery on your computer and scan the QR code.'**
  String get noMachinesHint;

  /// No description provided for @forgetTitle.
  ///
  /// In en, this message translates to:
  /// **'Remove {name}?'**
  String forgetTitle(String name);

  /// No description provided for @forgetBody.
  ///
  /// In en, this message translates to:
  /// **'This phone forgets the computer and its cached data. You can pair again later.'**
  String get forgetBody;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @forget.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get forget;

  /// No description provided for @lastSeenAt.
  ///
  /// In en, this message translates to:
  /// **'Old data, {time}'**
  String lastSeenAt(String time);

  /// No description provided for @panesCount.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{No panes} =1{1 pane} other{{count} panes}}'**
  String panesCount(int count);

  /// No description provided for @needsYouCount.
  ///
  /// In en, this message translates to:
  /// **'{count} need you'**
  String needsYouCount(int count);

  /// No description provided for @runningCount.
  ///
  /// In en, this message translates to:
  /// **'{count} running'**
  String runningCount(int count);

  /// No description provided for @doneCount.
  ///
  /// In en, this message translates to:
  /// **'{count} done'**
  String doneCount(int count);

  /// No description provided for @subagentsCount.
  ///
  /// In en, this message translates to:
  /// **'{count} subagents'**
  String subagentsCount(int count);

  /// No description provided for @statusWorking.
  ///
  /// In en, this message translates to:
  /// **'Running'**
  String get statusWorking;

  /// No description provided for @statusSubagents.
  ///
  /// In en, this message translates to:
  /// **'Subagents running'**
  String get statusSubagents;

  /// No description provided for @statusAsked.
  ///
  /// In en, this message translates to:
  /// **'Needs permission'**
  String get statusAsked;

  /// No description provided for @statusFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get statusFailed;

  /// No description provided for @statusDone.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get statusDone;

  /// No description provided for @statusStarting.
  ///
  /// In en, this message translates to:
  /// **'Starting'**
  String get statusStarting;

  /// No description provided for @statusEnded.
  ///
  /// In en, this message translates to:
  /// **'Ended'**
  String get statusEnded;

  /// No description provided for @statusParked.
  ///
  /// In en, this message translates to:
  /// **'Parked'**
  String get statusParked;

  /// No description provided for @statusIdle.
  ///
  /// In en, this message translates to:
  /// **'Idle'**
  String get statusIdle;

  /// No description provided for @linkIdle.
  ///
  /// In en, this message translates to:
  /// **'Not connected'**
  String get linkIdle;

  /// No description provided for @linkDiscovering.
  ///
  /// In en, this message translates to:
  /// **'Looking for computer'**
  String get linkDiscovering;

  /// No description provided for @linkHandshaking.
  ///
  /// In en, this message translates to:
  /// **'Connecting'**
  String get linkHandshaking;

  /// No description provided for @linkLan.
  ///
  /// In en, this message translates to:
  /// **'LAN'**
  String get linkLan;

  /// No description provided for @linkRelay.
  ///
  /// In en, this message translates to:
  /// **'Relay'**
  String get linkRelay;

  /// No description provided for @linkReconnecting.
  ///
  /// In en, this message translates to:
  /// **'Reconnecting, try {attempt}'**
  String linkReconnecting(int attempt);

  /// No description provided for @linkOffline.
  ///
  /// In en, this message translates to:
  /// **'Offline'**
  String get linkOffline;

  /// No description provided for @linkProtoTooOld.
  ///
  /// In en, this message translates to:
  /// **'Update needed'**
  String get linkProtoTooOld;

  /// No description provided for @linkRevoked.
  ///
  /// In en, this message translates to:
  /// **'Revoked'**
  String get linkRevoked;

  /// No description provided for @reasonDesktopAbsent.
  ///
  /// In en, this message translates to:
  /// **'The computer is not reachable through the relay.'**
  String get reasonDesktopAbsent;

  /// No description provided for @reasonRemoteOff.
  ///
  /// In en, this message translates to:
  /// **'Remote is turned off on the computer.'**
  String get reasonRemoteOff;

  /// No description provided for @reasonRestarting.
  ///
  /// In en, this message translates to:
  /// **'The computer is restarting Orrery.'**
  String get reasonRestarting;

  /// No description provided for @reasonTokenExpired.
  ///
  /// In en, this message translates to:
  /// **'Relay access expired. Join the same Wi-Fi once to renew it.'**
  String get reasonTokenExpired;

  /// No description provided for @reasonRateLimited.
  ///
  /// In en, this message translates to:
  /// **'Too many attempts. Waiting before retrying.'**
  String get reasonRateLimited;

  /// No description provided for @reasonNetworkLost.
  ///
  /// In en, this message translates to:
  /// **'No network on this phone.'**
  String get reasonNetworkLost;

  /// No description provided for @reasonNoRoute.
  ///
  /// In en, this message translates to:
  /// **'No route to the computer.'**
  String get reasonNoRoute;

  /// No description provided for @reasonHandshakeFailed.
  ///
  /// In en, this message translates to:
  /// **'Secure handshake failed.'**
  String get reasonHandshakeFailed;

  /// No description provided for @offlineBanner.
  ///
  /// In en, this message translates to:
  /// **'Computer offline, data from {time}'**
  String offlineBanner(String time);

  /// No description provided for @offlineNoData.
  ///
  /// In en, this message translates to:
  /// **'Computer offline, no saved data'**
  String get offlineNoData;

  /// No description provided for @retry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get retry;

  /// No description provided for @errDenied.
  ///
  /// In en, this message translates to:
  /// **'The computer refused this action.'**
  String get errDenied;

  /// No description provided for @errBusy.
  ///
  /// In en, this message translates to:
  /// **'The pane is busy. Try again in a moment.'**
  String get errBusy;

  /// No description provided for @errLimit.
  ///
  /// In en, this message translates to:
  /// **'Limit reached.'**
  String get errLimit;

  /// No description provided for @errNotFound.
  ///
  /// In en, this message translates to:
  /// **'Not found. It may have been closed.'**
  String get errNotFound;

  /// No description provided for @errInvalid.
  ///
  /// In en, this message translates to:
  /// **'Invalid request.'**
  String get errInvalid;

  /// No description provided for @errTimeout.
  ///
  /// In en, this message translates to:
  /// **'The computer did not answer in time.'**
  String get errTimeout;

  /// No description provided for @errOffline.
  ///
  /// In en, this message translates to:
  /// **'Computer offline.'**
  String get errOffline;

  /// No description provided for @errRate.
  ///
  /// In en, this message translates to:
  /// **'Too many commands. Slow down.'**
  String get errRate;

  /// No description provided for @errProto.
  ///
  /// In en, this message translates to:
  /// **'App and desktop versions do not match.'**
  String get errProto;

  /// No description provided for @errInternal.
  ///
  /// In en, this message translates to:
  /// **'Something went wrong on the computer.'**
  String get errInternal;

  /// No description provided for @revokedTitle.
  ///
  /// In en, this message translates to:
  /// **'The computer revoked this phone'**
  String get revokedTitle;

  /// No description provided for @revokedBody.
  ///
  /// In en, this message translates to:
  /// **'Saved data was deleted. Pair again from Settings > Remote on the computer.'**
  String get revokedBody;

  /// No description provided for @removeMachine.
  ///
  /// In en, this message translates to:
  /// **'Remove computer'**
  String get removeMachine;

  /// No description provided for @protoTitle.
  ///
  /// In en, this message translates to:
  /// **'Update the app or the desktop'**
  String get protoTitle;

  /// No description provided for @protoBody.
  ///
  /// In en, this message translates to:
  /// **'This app and Orrery on the computer speak different protocol versions.'**
  String get protoBody;

  /// No description provided for @back.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get back;

  /// No description provided for @confirmMore.
  ///
  /// In en, this message translates to:
  /// **'{count} more waiting'**
  String confirmMore(int count);

  /// No description provided for @confirmExpires.
  ///
  /// In en, this message translates to:
  /// **'Cancels itself in {seconds} s'**
  String confirmExpires(int seconds);

  /// No description provided for @missionControl.
  ///
  /// In en, this message translates to:
  /// **'Mission Control'**
  String get missionControl;

  /// No description provided for @inbox.
  ///
  /// In en, this message translates to:
  /// **'Inbox'**
  String get inbox;

  /// No description provided for @more.
  ///
  /// In en, this message translates to:
  /// **'More'**
  String get more;

  /// No description provided for @reopenPane.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{Reopen closed pane} other{Reopen closed pane ({count})}}'**
  String reopenPane(int count);

  /// No description provided for @openPane.
  ///
  /// In en, this message translates to:
  /// **'Open pane'**
  String get openPane;

  /// No description provided for @workspaceLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading workspace'**
  String get workspaceLoading;

  /// No description provided for @workspaceEmpty.
  ///
  /// In en, this message translates to:
  /// **'Workspace is empty'**
  String get workspaceEmpty;

  /// No description provided for @workspaceEmptyHint.
  ///
  /// In en, this message translates to:
  /// **'Open a pane to start working from your phone.'**
  String get workspaceEmptyHint;

  /// No description provided for @otherProject.
  ///
  /// In en, this message translates to:
  /// **'No project'**
  String get otherProject;

  /// No description provided for @inboxEmpty.
  ///
  /// In en, this message translates to:
  /// **'Nothing needs you'**
  String get inboxEmpty;

  /// No description provided for @inboxEmptyHint.
  ///
  /// In en, this message translates to:
  /// **'Permission requests and finished work show up here.'**
  String get inboxEmptyHint;

  /// No description provided for @inboxWaiting.
  ///
  /// In en, this message translates to:
  /// **'waiting for you'**
  String get inboxWaiting;

  /// No description provided for @inboxDone.
  ///
  /// In en, this message translates to:
  /// **'finished'**
  String get inboxDone;

  /// No description provided for @filterAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get filterAll;

  /// No description provided for @filterNeeds.
  ///
  /// In en, this message translates to:
  /// **'Needs you'**
  String get filterNeeds;

  /// No description provided for @filterRunning.
  ///
  /// In en, this message translates to:
  /// **'Running'**
  String get filterRunning;

  /// No description provided for @filterDone.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get filterDone;

  /// No description provided for @filterIdle.
  ///
  /// In en, this message translates to:
  /// **'Idle'**
  String get filterIdle;

  /// No description provided for @missionEmpty.
  ///
  /// In en, this message translates to:
  /// **'No pane matches this filter'**
  String get missionEmpty;

  /// No description provided for @tabClosed.
  ///
  /// In en, this message translates to:
  /// **'This tab was closed on the computer'**
  String get tabClosed;

  /// No description provided for @linkUnknownMachine.
  ///
  /// In en, this message translates to:
  /// **'This computer is not paired on this phone'**
  String get linkUnknownMachine;

  /// No description provided for @linkPaneGone.
  ///
  /// In en, this message translates to:
  /// **'That pane is no longer open'**
  String get linkPaneGone;

  /// No description provided for @paneClosed.
  ///
  /// In en, this message translates to:
  /// **'The pane you were viewing was closed'**
  String get paneClosed;

  /// No description provided for @tabActions.
  ///
  /// In en, this message translates to:
  /// **'Tab actions'**
  String get tabActions;

  /// No description provided for @renameTab.
  ///
  /// In en, this message translates to:
  /// **'Rename tab'**
  String get renameTab;

  /// No description provided for @renameSession.
  ///
  /// In en, this message translates to:
  /// **'Rename session'**
  String get renameSession;

  /// No description provided for @renameSessionHint.
  ///
  /// In en, this message translates to:
  /// **'Empty = original title'**
  String get renameSessionHint;

  /// No description provided for @renameMachine.
  ///
  /// In en, this message translates to:
  /// **'Rename computer'**
  String get renameMachine;

  /// No description provided for @renameMachineHint.
  ///
  /// In en, this message translates to:
  /// **'Empty = name set on the computer'**
  String get renameMachineHint;

  /// No description provided for @machineOriginal.
  ///
  /// In en, this message translates to:
  /// **'Named {name} on the computer'**
  String machineOriginal(String name);

  /// No description provided for @renameHint.
  ///
  /// In en, this message translates to:
  /// **'Empty = default name'**
  String get renameHint;

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @movePane.
  ///
  /// In en, this message translates to:
  /// **'Move pane to another tab'**
  String get movePane;

  /// No description provided for @compact.
  ///
  /// In en, this message translates to:
  /// **'Compact'**
  String get compact;

  /// No description provided for @compactTitle.
  ///
  /// In en, this message translates to:
  /// **'Compact this agent?'**
  String get compactTitle;

  /// No description provided for @compactBody.
  ///
  /// In en, this message translates to:
  /// **'Orrery sends the handoff prompt and /compact into this pane.'**
  String get compactBody;

  /// No description provided for @closePane.
  ///
  /// In en, this message translates to:
  /// **'Close pane'**
  String get closePane;

  /// No description provided for @closePaneTitle.
  ///
  /// In en, this message translates to:
  /// **'Close {title}?'**
  String closePaneTitle(String title);

  /// No description provided for @closePaneBody.
  ///
  /// In en, this message translates to:
  /// **'The pane closes on the computer too. You can reopen it from the menu.'**
  String get closePaneBody;

  /// No description provided for @tapToView.
  ///
  /// In en, this message translates to:
  /// **'Tap to view'**
  String get tapToView;

  /// No description provided for @typerYou.
  ///
  /// In en, this message translates to:
  /// **'You are typing'**
  String get typerYou;

  /// No description provided for @typerViewOnly.
  ///
  /// In en, this message translates to:
  /// **'View only'**
  String get typerViewOnly;

  /// No description provided for @typeDirect.
  ///
  /// In en, this message translates to:
  /// **'Type straight into the terminal'**
  String get typeDirect;

  /// No description provided for @inputHint.
  ///
  /// In en, this message translates to:
  /// **'Message or command'**
  String get inputHint;

  /// No description provided for @inputOffline.
  ///
  /// In en, this message translates to:
  /// **'Offline, cannot type'**
  String get inputOffline;

  /// No description provided for @send.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get send;

  /// No description provided for @answerYes.
  ///
  /// In en, this message translates to:
  /// **'Yes'**
  String get answerYes;

  /// No description provided for @answerYesAlways.
  ///
  /// In en, this message translates to:
  /// **'Yes, always'**
  String get answerYesAlways;

  /// No description provided for @answerNo.
  ///
  /// In en, this message translates to:
  /// **'No'**
  String get answerNo;

  /// No description provided for @answerEsc.
  ///
  /// In en, this message translates to:
  /// **'Esc'**
  String get answerEsc;

  /// No description provided for @paneNotTerminal.
  ///
  /// In en, this message translates to:
  /// **'This pane is not a terminal. Open it on the computer.'**
  String get paneNotTerminal;

  /// No description provided for @attachLimit.
  ///
  /// In en, this message translates to:
  /// **'Only 4 terminals can be watched at once. Close one first.'**
  String get attachLimit;

  /// No description provided for @sessionEnded.
  ///
  /// In en, this message translates to:
  /// **'Session ended (code {code})'**
  String sessionEnded(int code);

  /// No description provided for @sessionLost.
  ///
  /// In en, this message translates to:
  /// **'Lost the terminal on the computer'**
  String get sessionLost;

  /// No description provided for @pickProject.
  ///
  /// In en, this message translates to:
  /// **'Project'**
  String get pickProject;

  /// No description provided for @noProjects.
  ///
  /// In en, this message translates to:
  /// **'No project yet. Add favorites on the computer first.'**
  String get noProjects;

  /// No description provided for @pickAgent.
  ///
  /// In en, this message translates to:
  /// **'Open with'**
  String get pickAgent;

  /// No description provided for @shell.
  ///
  /// In en, this message translates to:
  /// **'Shell'**
  String get shell;

  /// No description provided for @promptOptional.
  ///
  /// In en, this message translates to:
  /// **'Prompt (optional)'**
  String get promptOptional;

  /// No description provided for @promptHint.
  ///
  /// In en, this message translates to:
  /// **'What should the agent do first?'**
  String get promptHint;

  /// No description provided for @submitNow.
  ///
  /// In en, this message translates to:
  /// **'Send it right away'**
  String get submitNow;

  /// No description provided for @submitNowHint.
  ///
  /// In en, this message translates to:
  /// **'Off: the prompt is only typed in, you press Enter later.'**
  String get submitNowHint;

  /// No description provided for @openPaneAction.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get openPaneAction;

  /// No description provided for @paneOpened.
  ///
  /// In en, this message translates to:
  /// **'Pane opened'**
  String get paneOpened;

  /// No description provided for @pairBadQr.
  ///
  /// In en, this message translates to:
  /// **'Invalid QR code'**
  String get pairBadQr;

  /// No description provided for @pairExpired.
  ///
  /// In en, this message translates to:
  /// **'QR code expired. Create a new one on the computer.'**
  String get pairExpired;

  /// No description provided for @pairNoRoute.
  ///
  /// In en, this message translates to:
  /// **'Cannot find the computer on this Wi-Fi.'**
  String get pairNoRoute;

  /// No description provided for @pairNetwork.
  ///
  /// In en, this message translates to:
  /// **'Connection lost while pairing. Scan again.'**
  String get pairNetwork;

  /// No description provided for @pairKeyMismatch.
  ///
  /// In en, this message translates to:
  /// **'Computer key does not match the QR code. Pairing stopped.'**
  String get pairKeyMismatch;

  /// No description provided for @pairTimeout.
  ///
  /// In en, this message translates to:
  /// **'The computer did not answer in time.'**
  String get pairTimeout;

  /// No description provided for @pairDenied.
  ///
  /// In en, this message translates to:
  /// **'The computer refused the pairing.'**
  String get pairDenied;

  /// No description provided for @pairProtocol.
  ///
  /// In en, this message translates to:
  /// **'Protocol error while pairing.'**
  String get pairProtocol;

  /// No description provided for @pairing.
  ///
  /// In en, this message translates to:
  /// **'Pairing...'**
  String get pairing;

  /// No description provided for @pushTitle.
  ///
  /// In en, this message translates to:
  /// **'Notifications'**
  String get pushTitle;

  /// No description provided for @pushHint.
  ///
  /// In en, this message translates to:
  /// **'Alert me when a pane needs me while the app is closed.'**
  String get pushHint;

  /// No description provided for @pushNotSetUp.
  ///
  /// In en, this message translates to:
  /// **'Not set up on the desktop. Add a Firebase project in Orrery Settings > Remote.'**
  String get pushNotSetUp;

  /// No description provided for @pushForeign.
  ///
  /// In en, this message translates to:
  /// **'This computer uses a different Firebase project; notifications are off for it'**
  String get pushForeign;

  /// No description provided for @pushRevoked.
  ///
  /// In en, this message translates to:
  /// **'Notification permission is off in the system settings.'**
  String get pushRevoked;

  /// No description provided for @pushDenied.
  ///
  /// In en, this message translates to:
  /// **'Notification permission was refused. Allow it in the system settings.'**
  String get pushDenied;

  /// No description provided for @pushUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Notifications are not available in this build.'**
  String get pushUnavailable;

  /// No description provided for @pairScanHint.
  ///
  /// In en, this message translates to:
  /// **'Scan the QR code in Orrery Settings'**
  String get pairScanHint;

  /// No description provided for @navWorkspace.
  ///
  /// In en, this message translates to:
  /// **'Workspace'**
  String get navWorkspace;

  /// No description provided for @vMonitor.
  ///
  /// In en, this message translates to:
  /// **'Monitor'**
  String get vMonitor;

  /// No description provided for @vMonitorEmpty.
  ///
  /// In en, this message translates to:
  /// **'No sessions on the computer right now.'**
  String get vMonitorEmpty;

  /// No description provided for @vTokens.
  ///
  /// In en, this message translates to:
  /// **'{count} tokens'**
  String vTokens(String count);

  /// No description provided for @vContextPct.
  ///
  /// In en, this message translates to:
  /// **'context {percent}%'**
  String vContextPct(int percent);

  /// No description provided for @vNotInWorkspace.
  ///
  /// In en, this message translates to:
  /// **'This session is not open in the Workspace.'**
  String get vNotInWorkspace;

  /// No description provided for @vViewerMenu.
  ///
  /// In en, this message translates to:
  /// **'Explorer, Changes, Git Log, Run'**
  String get vViewerMenu;

  /// No description provided for @vExplorer.
  ///
  /// In en, this message translates to:
  /// **'Explorer'**
  String get vExplorer;

  /// No description provided for @vChanges.
  ///
  /// In en, this message translates to:
  /// **'Changes'**
  String get vChanges;

  /// No description provided for @vGitLog.
  ///
  /// In en, this message translates to:
  /// **'Git Log'**
  String get vGitLog;

  /// No description provided for @vRun.
  ///
  /// In en, this message translates to:
  /// **'Run'**
  String get vRun;

  /// No description provided for @vRetry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get vRetry;

  /// No description provided for @vErrNotRepo.
  ///
  /// In en, this message translates to:
  /// **'This folder is not a git repository.'**
  String get vErrNotRepo;

  /// No description provided for @vErrConflict.
  ///
  /// In en, this message translates to:
  /// **'The file changed on the computer.'**
  String get vErrConflict;

  /// No description provided for @vErrMissing.
  ///
  /// In en, this message translates to:
  /// **'The file no longer exists.'**
  String get vErrMissing;

  /// No description provided for @vErrTooLarge.
  ///
  /// In en, this message translates to:
  /// **'Too large to handle on the phone.'**
  String get vErrTooLarge;

  /// No description provided for @vErrExists.
  ///
  /// In en, this message translates to:
  /// **'That name already exists.'**
  String get vErrExists;

  /// No description provided for @vErrNotMerged.
  ///
  /// In en, this message translates to:
  /// **'The branch is not merged yet.'**
  String get vErrNotMerged;

  /// No description provided for @vErrCurrent.
  ///
  /// In en, this message translates to:
  /// **'This is the current branch.'**
  String get vErrCurrent;

  /// No description provided for @vErrNoUpstream.
  ///
  /// In en, this message translates to:
  /// **'The branch has no upstream.'**
  String get vErrNoUpstream;

  /// No description provided for @vErrNonFf.
  ///
  /// In en, this message translates to:
  /// **'Cannot fast-forward. Update from the computer.'**
  String get vErrNonFf;

  /// No description provided for @vErrAuth.
  ///
  /// In en, this message translates to:
  /// **'The remote refused the credentials.'**
  String get vErrAuth;

  /// No description provided for @vErrNothing.
  ///
  /// In en, this message translates to:
  /// **'Nothing to commit.'**
  String get vErrNothing;

  /// No description provided for @vErrEmptyMessage.
  ///
  /// In en, this message translates to:
  /// **'Write a commit message first.'**
  String get vErrEmptyMessage;

  /// No description provided for @vErrPushed.
  ///
  /// In en, this message translates to:
  /// **'That commit is already pushed.'**
  String get vErrPushed;

  /// No description provided for @vErrHook.
  ///
  /// In en, this message translates to:
  /// **'A git hook rejected the commit.'**
  String get vErrHook;

  /// No description provided for @vErrFailed.
  ///
  /// In en, this message translates to:
  /// **'The command failed.'**
  String get vErrFailed;

  /// No description provided for @vCancelled.
  ///
  /// In en, this message translates to:
  /// **'Cancelled.'**
  String get vCancelled;

  /// No description provided for @vWaitConfirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm on this phone to continue.'**
  String get vWaitConfirm;

  /// No description provided for @vFindFile.
  ///
  /// In en, this message translates to:
  /// **'Find file'**
  String get vFindFile;

  /// No description provided for @vFindFileHint.
  ///
  /// In en, this message translates to:
  /// **'Type part of a file name or path.'**
  String get vFindFileHint;

  /// No description provided for @vFindInFile.
  ///
  /// In en, this message translates to:
  /// **'Find in file'**
  String get vFindInFile;

  /// No description provided for @vNoMatches.
  ///
  /// In en, this message translates to:
  /// **'No matches'**
  String get vNoMatches;

  /// No description provided for @vPrevMatch.
  ///
  /// In en, this message translates to:
  /// **'Previous match'**
  String get vPrevMatch;

  /// No description provided for @vNextMatch.
  ///
  /// In en, this message translates to:
  /// **'Next match'**
  String get vNextMatch;

  /// No description provided for @vListTruncated.
  ///
  /// In en, this message translates to:
  /// **'The list is cut at the computer limit.'**
  String get vListTruncated;

  /// No description provided for @vFolderEmpty.
  ///
  /// In en, this message translates to:
  /// **'This folder is empty.'**
  String get vFolderEmpty;

  /// No description provided for @vEdit.
  ///
  /// In en, this message translates to:
  /// **'Edit'**
  String get vEdit;

  /// No description provided for @vSaved.
  ///
  /// In en, this message translates to:
  /// **'Saved'**
  String get vSaved;

  /// No description provided for @vDiscardEditsTitle.
  ///
  /// In en, this message translates to:
  /// **'Discard your edits?'**
  String get vDiscardEditsTitle;

  /// No description provided for @vDiscardEditsBody.
  ///
  /// In en, this message translates to:
  /// **'The changes you typed are not saved.'**
  String get vDiscardEditsBody;

  /// No description provided for @vKeepEditing.
  ///
  /// In en, this message translates to:
  /// **'Keep editing'**
  String get vKeepEditing;

  /// No description provided for @vDiscard.
  ///
  /// In en, this message translates to:
  /// **'Discard'**
  String get vDiscard;

  /// No description provided for @vFileTooBigPhone.
  ///
  /// In en, this message translates to:
  /// **'This file is too large for the phone.'**
  String get vFileTooBigPhone;

  /// No description provided for @vFileBinary.
  ///
  /// In en, this message translates to:
  /// **'Binary file, cannot show it.'**
  String get vFileBinary;

  /// No description provided for @vFileImage.
  ///
  /// In en, this message translates to:
  /// **'Image file, open it on the computer.'**
  String get vFileImage;

  /// No description provided for @vFileTooLarge.
  ///
  /// In en, this message translates to:
  /// **'File is {size}, too large to open here.'**
  String vFileTooLarge(String size);

  /// No description provided for @vFileNotUtf8.
  ///
  /// In en, this message translates to:
  /// **'This file is not UTF-8 text, so it is read-only on the computer.'**
  String get vFileNotUtf8;

  /// No description provided for @vFileChangedOnDisk.
  ///
  /// In en, this message translates to:
  /// **'This file changed on the computer. Your edits were not saved.'**
  String get vFileChangedOnDisk;

  /// No description provided for @vCopied.
  ///
  /// In en, this message translates to:
  /// **'Copied'**
  String get vCopied;

  /// No description provided for @vCopyMine.
  ///
  /// In en, this message translates to:
  /// **'Copy my text'**
  String get vCopyMine;

  /// No description provided for @vReloadDiscard.
  ///
  /// In en, this message translates to:
  /// **'Reload, drop my edits'**
  String get vReloadDiscard;

  /// No description provided for @vDiffBinary.
  ///
  /// In en, this message translates to:
  /// **'Binary file, no text diff.'**
  String get vDiffBinary;

  /// No description provided for @vDiffTooLarge.
  ///
  /// In en, this message translates to:
  /// **'The diff is too large to show.'**
  String get vDiffTooLarge;

  /// No description provided for @vDiffEmpty.
  ///
  /// In en, this message translates to:
  /// **'No differences.'**
  String get vDiffEmpty;

  /// No description provided for @vUnchangedLines.
  ///
  /// In en, this message translates to:
  /// **'{count} unchanged lines'**
  String vUnchangedLines(int count);

  /// No description provided for @vChangesClean.
  ///
  /// In en, this message translates to:
  /// **'No changes. The working tree is clean.'**
  String get vChangesClean;

  /// No description provided for @vStaged.
  ///
  /// In en, this message translates to:
  /// **'Staged ({count})'**
  String vStaged(int count);

  /// No description provided for @vUnstaged.
  ///
  /// In en, this message translates to:
  /// **'Changes ({count})'**
  String vUnstaged(int count);

  /// No description provided for @vStageAll.
  ///
  /// In en, this message translates to:
  /// **'Stage all'**
  String get vStageAll;

  /// No description provided for @vUnstageAll.
  ///
  /// In en, this message translates to:
  /// **'Unstage all'**
  String get vUnstageAll;

  /// No description provided for @vStage.
  ///
  /// In en, this message translates to:
  /// **'Stage'**
  String get vStage;

  /// No description provided for @vUnstage.
  ///
  /// In en, this message translates to:
  /// **'Unstage'**
  String get vUnstage;

  /// No description provided for @vDiscardChanges.
  ///
  /// In en, this message translates to:
  /// **'Discard changes'**
  String get vDiscardChanges;

  /// No description provided for @vDetachedHead.
  ///
  /// In en, this message translates to:
  /// **'Detached HEAD'**
  String get vDetachedHead;

  /// No description provided for @vCommitMessage.
  ///
  /// In en, this message translates to:
  /// **'Commit message'**
  String get vCommitMessage;

  /// No description provided for @vCommit.
  ///
  /// In en, this message translates to:
  /// **'Commit'**
  String get vCommit;

  /// No description provided for @vCommitted.
  ///
  /// In en, this message translates to:
  /// **'Committed'**
  String get vCommitted;

  /// No description provided for @vNoCommits.
  ///
  /// In en, this message translates to:
  /// **'No commits yet.'**
  String get vNoCommits;

  /// No description provided for @vBranches.
  ///
  /// In en, this message translates to:
  /// **'Branches'**
  String get vBranches;

  /// No description provided for @vCommitDetail.
  ///
  /// In en, this message translates to:
  /// **'Commit'**
  String get vCommitDetail;

  /// No description provided for @vFilesChanged.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 file changed} other{{count} files changed}}'**
  String vFilesChanged(int count);

  /// No description provided for @vLocalBranches.
  ///
  /// In en, this message translates to:
  /// **'Local'**
  String get vLocalBranches;

  /// No description provided for @vRemoteBranches.
  ///
  /// In en, this message translates to:
  /// **'Remote'**
  String get vRemoteBranches;

  /// No description provided for @vTags.
  ///
  /// In en, this message translates to:
  /// **'Tags'**
  String get vTags;

  /// No description provided for @vFetch.
  ///
  /// In en, this message translates to:
  /// **'Fetch'**
  String get vFetch;

  /// No description provided for @vFetched.
  ///
  /// In en, this message translates to:
  /// **'Fetched'**
  String get vFetched;

  /// No description provided for @vNewBranch.
  ///
  /// In en, this message translates to:
  /// **'New branch'**
  String get vNewBranch;

  /// No description provided for @vBranchName.
  ///
  /// In en, this message translates to:
  /// **'Branch name'**
  String get vBranchName;

  /// No description provided for @vBranchFrom.
  ///
  /// In en, this message translates to:
  /// **'Created from {base}'**
  String vBranchFrom(String base);

  /// No description provided for @vCreate.
  ///
  /// In en, this message translates to:
  /// **'Create'**
  String get vCreate;

  /// No description provided for @vBranchCreated.
  ///
  /// In en, this message translates to:
  /// **'Created {name}'**
  String vBranchCreated(String name);

  /// No description provided for @vUpdateBranch.
  ///
  /// In en, this message translates to:
  /// **'Update (fast-forward)'**
  String get vUpdateBranch;

  /// No description provided for @vUpdated.
  ///
  /// In en, this message translates to:
  /// **'Updated'**
  String get vUpdated;

  /// No description provided for @vDeleteBranch.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get vDeleteBranch;

  /// No description provided for @vUpstreamGone.
  ///
  /// In en, this message translates to:
  /// **'upstream gone'**
  String get vUpstreamGone;

  /// No description provided for @vPushAction.
  ///
  /// In en, this message translates to:
  /// **'Push...'**
  String get vPushAction;

  /// No description provided for @vPushTitle.
  ///
  /// In en, this message translates to:
  /// **'Push {branch}'**
  String vPushTitle(String branch);

  /// No description provided for @vPushTarget.
  ///
  /// In en, this message translates to:
  /// **'To {target}'**
  String vPushTarget(String target);

  /// No description provided for @vPushOutgoing.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{Nothing new to send} =1{1 commit to send} other{{count} commits to send}}'**
  String vPushOutgoing(int count);

  /// No description provided for @vPushNewBranch.
  ///
  /// In en, this message translates to:
  /// **'Creates a new branch on the remote.'**
  String get vPushNewBranch;

  /// No description provided for @vAndMore.
  ///
  /// In en, this message translates to:
  /// **'and more...'**
  String get vAndMore;

  /// No description provided for @vPushed.
  ///
  /// In en, this message translates to:
  /// **'Pushed'**
  String get vPushed;

  /// No description provided for @vRunNoConfigs.
  ///
  /// In en, this message translates to:
  /// **'No run configuration for this project. Create one on the computer.'**
  String get vRunNoConfigs;

  /// No description provided for @vRunConfig.
  ///
  /// In en, this message translates to:
  /// **'Configuration'**
  String get vRunConfig;

  /// No description provided for @vRunDevice.
  ///
  /// In en, this message translates to:
  /// **'Device'**
  String get vRunDevice;

  /// No description provided for @vRunStart.
  ///
  /// In en, this message translates to:
  /// **'Run'**
  String get vRunStart;

  /// No description provided for @vRunHotReload.
  ///
  /// In en, this message translates to:
  /// **'Hot reload'**
  String get vRunHotReload;

  /// No description provided for @vRunRestart.
  ///
  /// In en, this message translates to:
  /// **'Restart'**
  String get vRunRestart;

  /// No description provided for @vRunStop.
  ///
  /// In en, this message translates to:
  /// **'Stop'**
  String get vRunStop;

  /// No description provided for @vRunNoLog.
  ///
  /// In en, this message translates to:
  /// **'No log yet'**
  String get vRunNoLog;

  /// No description provided for @lockReason.
  ///
  /// In en, this message translates to:
  /// **'Unlock Orrery Remote'**
  String get lockReason;

  /// No description provided for @lockTitle.
  ///
  /// In en, this message translates to:
  /// **'Orrery Remote is locked'**
  String get lockTitle;

  /// No description provided for @lockBody.
  ///
  /// In en, this message translates to:
  /// **'Verify with fingerprint, face or your phone\'s screen lock to continue.'**
  String get lockBody;

  /// No description provided for @lockUnlock.
  ///
  /// In en, this message translates to:
  /// **'Unlock'**
  String get lockUnlock;

  /// No description provided for @lockCancelled.
  ///
  /// In en, this message translates to:
  /// **'Verification was cancelled.'**
  String get lockCancelled;

  /// No description provided for @lockFailed.
  ///
  /// In en, this message translates to:
  /// **'Verification failed. Try again.'**
  String get lockFailed;

  /// No description provided for @lockNoCredentials.
  ///
  /// In en, this message translates to:
  /// **'This phone has no screen lock. Set a PIN, pattern, password or fingerprint in the system settings.'**
  String get lockNoCredentials;

  /// No description provided for @lockOpenSecurity.
  ///
  /// In en, this message translates to:
  /// **'Open security settings'**
  String get lockOpenSecurity;

  /// No description provided for @lockTurnOff.
  ///
  /// In en, this message translates to:
  /// **'Turn off app lock'**
  String get lockTurnOff;

  /// No description provided for @lockTurnOffTitle.
  ///
  /// In en, this message translates to:
  /// **'Turn off app lock?'**
  String get lockTurnOffTitle;

  /// No description provided for @lockTurnOffWarn.
  ///
  /// In en, this message translates to:
  /// **'Anyone who holds this phone can then open your sessions and terminals.'**
  String get lockTurnOffWarn;

  /// No description provided for @lockSettingsTitle.
  ///
  /// In en, this message translates to:
  /// **'App lock'**
  String get lockSettingsTitle;

  /// No description provided for @lockSwitch.
  ///
  /// In en, this message translates to:
  /// **'Require verification'**
  String get lockSwitch;

  /// No description provided for @lockSwitchHint.
  ///
  /// In en, this message translates to:
  /// **'Fingerprint, face or the phone screen lock.'**
  String get lockSwitchHint;

  /// No description provided for @lockAfter.
  ///
  /// In en, this message translates to:
  /// **'Ask again after'**
  String get lockAfter;

  /// No description provided for @lockEveryOpen.
  ///
  /// In en, this message translates to:
  /// **'Every time the app opens'**
  String get lockEveryOpen;

  /// No description provided for @lockMinutes15.
  ///
  /// In en, this message translates to:
  /// **'15 minutes'**
  String get lockMinutes15;

  /// No description provided for @lockHour1.
  ///
  /// In en, this message translates to:
  /// **'1 hour (default)'**
  String get lockHour1;

  /// No description provided for @lockHours4.
  ///
  /// In en, this message translates to:
  /// **'4 hours'**
  String get lockHours4;

  /// No description provided for @lockSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not save the setting.'**
  String get lockSaveFailed;

  /// No description provided for @homeTitle.
  ///
  /// In en, this message translates to:
  /// **'My computers'**
  String get homeTitle;

  /// No description provided for @homeConnected.
  ///
  /// In en, this message translates to:
  /// **'Connected'**
  String get homeConnected;

  /// No description provided for @homeDuplicates.
  ///
  /// In en, this message translates to:
  /// **'Same name'**
  String get homeDuplicates;

  /// No description provided for @idleCount.
  ///
  /// In en, this message translates to:
  /// **'{count} idle'**
  String idleCount(int count);

  /// No description provided for @pairedOn.
  ///
  /// In en, this message translates to:
  /// **'Paired {date}'**
  String pairedOn(String date);

  /// No description provided for @cleanOld.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Remove 1 old copy} other{Remove {count} old copies}}'**
  String cleanOld(int count);

  /// No description provided for @cleanOldTitle.
  ///
  /// In en, this message translates to:
  /// **'Remove old copies?'**
  String get cleanOldTitle;

  /// No description provided for @cleanOldBody.
  ///
  /// In en, this message translates to:
  /// **'This phone forgets {count} copies that share a name with a newer one. The newer one stays.'**
  String cleanOldBody(int count);

  /// No description provided for @cleanOldDone.
  ///
  /// In en, this message translates to:
  /// **'Old copies removed.'**
  String get cleanOldDone;

  /// No description provided for @pairNewMachine.
  ///
  /// In en, this message translates to:
  /// **'Pair a new computer'**
  String get pairNewMachine;

  /// No description provided for @enterCodeManual.
  ///
  /// In en, this message translates to:
  /// **'Enter code manually'**
  String get enterCodeManual;

  /// No description provided for @pairTabScan.
  ///
  /// In en, this message translates to:
  /// **'Scan QR'**
  String get pairTabScan;

  /// No description provided for @pairTabCode.
  ///
  /// In en, this message translates to:
  /// **'Enter code'**
  String get pairTabCode;

  /// No description provided for @pairScanTitle.
  ///
  /// In en, this message translates to:
  /// **'Point the camera at the QR code'**
  String get pairScanTitle;

  /// No description provided for @pairE2eTitle.
  ///
  /// In en, this message translates to:
  /// **'End-to-end encrypted'**
  String get pairE2eTitle;

  /// No description provided for @pairE2eBody.
  ///
  /// In en, this message translates to:
  /// **'This phone and the computer share a Noise key. The relay only carries encrypted bytes.'**
  String get pairE2eBody;

  /// No description provided for @pairCodeLabel.
  ///
  /// In en, this message translates to:
  /// **'Pairing code from the computer'**
  String get pairCodeLabel;

  /// No description provided for @pairCodeField.
  ///
  /// In en, this message translates to:
  /// **'Pairing code'**
  String get pairCodeField;

  /// No description provided for @pairCodeSubmit.
  ///
  /// In en, this message translates to:
  /// **'Pair'**
  String get pairCodeSubmit;

  /// No description provided for @chipWaiting.
  ///
  /// In en, this message translates to:
  /// **'Waiting for you'**
  String get chipWaiting;

  /// No description provided for @monFilterAll.
  ///
  /// In en, this message translates to:
  /// **'All {count}'**
  String monFilterAll(int count);

  /// No description provided for @monFilterRunning.
  ///
  /// In en, this message translates to:
  /// **'Running {count}'**
  String monFilterRunning(int count);

  /// No description provided for @monFilterWaiting.
  ///
  /// In en, this message translates to:
  /// **'Waiting {count}'**
  String monFilterWaiting(int count);

  /// No description provided for @monFilterIdle.
  ///
  /// In en, this message translates to:
  /// **'Idle {count}'**
  String monFilterIdle(int count);

  /// No description provided for @monFilterEmpty.
  ///
  /// In en, this message translates to:
  /// **'No session in this filter'**
  String get monFilterEmpty;

  /// No description provided for @viewModeToCompact.
  ///
  /// In en, this message translates to:
  /// **'Group by state'**
  String get viewModeToCompact;

  /// No description provided for @viewModeToCards.
  ///
  /// In en, this message translates to:
  /// **'Show as cards'**
  String get viewModeToCards;

  /// No description provided for @viewModeToGrid.
  ///
  /// In en, this message translates to:
  /// **'Show all panes in a grid'**
  String get viewModeToGrid;

  /// No description provided for @viewModeToSingle.
  ///
  /// In en, this message translates to:
  /// **'Show one pane at a time'**
  String get viewModeToSingle;

  /// No description provided for @detailSubagents.
  ///
  /// In en, this message translates to:
  /// **'Sub-agents'**
  String get detailSubagents;

  /// No description provided for @detailStarted.
  ///
  /// In en, this message translates to:
  /// **'Since {time}'**
  String detailStarted(String time);

  /// No description provided for @detailOpenTerminal.
  ///
  /// In en, this message translates to:
  /// **'Open terminal'**
  String get detailOpenTerminal;

  /// No description provided for @detailNotInWorkspace.
  ///
  /// In en, this message translates to:
  /// **'This session is not open in the Workspace'**
  String get detailNotInWorkspace;

  /// No description provided for @detailTool.
  ///
  /// In en, this message translates to:
  /// **'Running tool'**
  String get detailTool;

  /// No description provided for @detailWaiting.
  ///
  /// In en, this message translates to:
  /// **'Waiting for'**
  String get detailWaiting;

  /// No description provided for @detailContext.
  ///
  /// In en, this message translates to:
  /// **'Context'**
  String get detailContext;

  /// No description provided for @detailNoSubagents.
  ///
  /// In en, this message translates to:
  /// **'No sub-agents'**
  String get detailNoSubagents;

  /// No description provided for @wsTitle.
  ///
  /// In en, this message translates to:
  /// **'Workspace'**
  String get wsTitle;

  /// No description provided for @wsOpenFull.
  ///
  /// In en, this message translates to:
  /// **'Open full screen'**
  String get wsOpenFull;

  /// No description provided for @wsPaneHint.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 pane in this tab} other{{count} panes in this tab}} · swipe to switch'**
  String wsPaneHint(int count);

  /// No description provided for @wsNoTabs.
  ///
  /// In en, this message translates to:
  /// **'No tab open'**
  String get wsNoTabs;

  /// No description provided for @wsAllTabs.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get wsAllTabs;

  /// No description provided for @paneSheetTitle.
  ///
  /// In en, this message translates to:
  /// **'Panes in tab {tab}'**
  String paneSheetTitle(String tab);

  /// No description provided for @paneSheetOpen.
  ///
  /// In en, this message translates to:
  /// **'Switch pane'**
  String get paneSheetOpen;

  /// No description provided for @diffAdded.
  ///
  /// In en, this message translates to:
  /// **'{count} added'**
  String diffAdded(int count);

  /// No description provided for @diffRemoved.
  ///
  /// In en, this message translates to:
  /// **'{count} removed'**
  String diffRemoved(int count);

  /// No description provided for @fileLines.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 line} other{{count} lines}}'**
  String fileLines(int count);

  /// No description provided for @fileReadOnly.
  ///
  /// In en, this message translates to:
  /// **'read only'**
  String get fileReadOnly;

  /// No description provided for @fileEditing.
  ///
  /// In en, this message translates to:
  /// **'editing'**
  String get fileEditing;

  /// No description provided for @fileUnsaved.
  ///
  /// In en, this message translates to:
  /// **'Unsaved'**
  String get fileUnsaved;

  /// No description provided for @navSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get navSettings;

  /// No description provided for @settingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settingsTitle;

  /// No description provided for @settingsSecurity.
  ///
  /// In en, this message translates to:
  /// **'Security'**
  String get settingsSecurity;

  /// No description provided for @settingsAppearance.
  ///
  /// In en, this message translates to:
  /// **'Appearance'**
  String get settingsAppearance;

  /// No description provided for @settingsLanguage.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get settingsLanguage;

  /// No description provided for @settingsTheme.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get settingsTheme;

  /// No description provided for @settingsFollowsMachine.
  ///
  /// In en, this message translates to:
  /// **'Follows the computer'**
  String get settingsFollowsMachine;

  /// No description provided for @settingsAbout.
  ///
  /// In en, this message translates to:
  /// **'About'**
  String get settingsAbout;

  /// No description provided for @settingsAppName.
  ///
  /// In en, this message translates to:
  /// **'Orrery Remote'**
  String get settingsAppName;

  /// No description provided for @settingsAppHint.
  ///
  /// In en, this message translates to:
  /// **'Remote for the Orrery app on your computer'**
  String get settingsAppHint;

  /// No description provided for @settingsMachine.
  ///
  /// In en, this message translates to:
  /// **'Connected computer'**
  String get settingsMachine;

  /// No description provided for @lockShortEvery.
  ///
  /// In en, this message translates to:
  /// **'Always'**
  String get lockShortEvery;

  /// No description provided for @lockShortMinutes15.
  ///
  /// In en, this message translates to:
  /// **'15 min'**
  String get lockShortMinutes15;

  /// No description provided for @lockShortHour1.
  ///
  /// In en, this message translates to:
  /// **'1 hour'**
  String get lockShortHour1;

  /// No description provided for @lockShortHours4.
  ///
  /// In en, this message translates to:
  /// **'4 hours'**
  String get lockShortHours4;

  /// No description provided for @settingsPushHint.
  ///
  /// In en, this message translates to:
  /// **'Choose which events reach this phone. The computer sends them while the app is closed.'**
  String get settingsPushHint;

  /// No description provided for @actTitle.
  ///
  /// In en, this message translates to:
  /// **'Recent activity'**
  String get actTitle;

  /// No description provided for @actEmpty.
  ///
  /// In en, this message translates to:
  /// **'No activity yet'**
  String get actEmpty;

  /// No description provided for @actStarted.
  ///
  /// In en, this message translates to:
  /// **'Session started'**
  String get actStarted;

  /// No description provided for @actEnded.
  ///
  /// In en, this message translates to:
  /// **'Session ended'**
  String get actEnded;

  /// No description provided for @actRenamed.
  ///
  /// In en, this message translates to:
  /// **'Renamed'**
  String get actRenamed;

  /// No description provided for @actStatus.
  ///
  /// In en, this message translates to:
  /// **'Status changed'**
  String get actStatus;

  /// No description provided for @actCleared.
  ///
  /// In en, this message translates to:
  /// **'Context cleared'**
  String get actCleared;

  /// No description provided for @actCompacted.
  ///
  /// In en, this message translates to:
  /// **'Compacted'**
  String get actCompacted;

  /// No description provided for @actAgentStarted.
  ///
  /// In en, this message translates to:
  /// **'Sub-agent started'**
  String get actAgentStarted;

  /// No description provided for @actAgentDone.
  ///
  /// In en, this message translates to:
  /// **'Sub-agent finished'**
  String get actAgentDone;

  /// No description provided for @actAgentFailed.
  ///
  /// In en, this message translates to:
  /// **'Sub-agent failed'**
  String get actAgentFailed;

  /// No description provided for @actAgentCancelled.
  ///
  /// In en, this message translates to:
  /// **'Sub-agent cancelled'**
  String get actAgentCancelled;

  /// No description provided for @actTool.
  ///
  /// In en, this message translates to:
  /// **'Tool'**
  String get actTool;

  /// No description provided for @actToolFailed.
  ///
  /// In en, this message translates to:
  /// **'Tool failed'**
  String get actToolFailed;

  /// No description provided for @actTurnStarted.
  ///
  /// In en, this message translates to:
  /// **'Turn started'**
  String get actTurnStarted;

  /// No description provided for @actTurnDone.
  ///
  /// In en, this message translates to:
  /// **'Turn finished'**
  String get actTurnDone;

  /// No description provided for @actTurnFailed.
  ///
  /// In en, this message translates to:
  /// **'Turn failed'**
  String get actTurnFailed;

  /// No description provided for @actTurnStopped.
  ///
  /// In en, this message translates to:
  /// **'Turn interrupted'**
  String get actTurnStopped;

  /// No description provided for @actSensitive.
  ///
  /// In en, this message translates to:
  /// **'Sensitive action'**
  String get actSensitive;

  /// No description provided for @pushGroupWaiting.
  ///
  /// In en, this message translates to:
  /// **'Session waiting for you'**
  String get pushGroupWaiting;

  /// No description provided for @pushGroupDone.
  ///
  /// In en, this message translates to:
  /// **'Session finished'**
  String get pushGroupDone;

  /// No description provided for @pushGroupError.
  ///
  /// In en, this message translates to:
  /// **'Error or unexpected exit'**
  String get pushGroupError;

  /// No description provided for @settingsThemeSystem.
  ///
  /// In en, this message translates to:
  /// **'Follows the phone'**
  String get settingsThemeSystem;

  /// No description provided for @settingsThemeLight.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get settingsThemeLight;

  /// No description provided for @settingsThemeDark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get settingsThemeDark;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'vi'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'vi':
      return AppLocalizationsVi();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
