import '../l10n/app_localizations.dart';
import '../ui/commands.dart';
import '../ui/viewers/viewer_api.dart';

String promptErrorText(AppLocalizations l, Object error) {
  if (error is ViewerFailure) {
    return switch (error.code) {
      'secret' => l.promptsErrSecret,
      'invalid-title' => l.promptsErrTitle,
      'invalid-body' => l.promptsErrBody,
      'too-many' => l.promptsErrFull,
      'not-found' => l.promptsErrNotFound,
      'denied' => l.errDenied,
      _ => l.promptsErrIo,
    };
  }
  return errorText(l, error);
}
