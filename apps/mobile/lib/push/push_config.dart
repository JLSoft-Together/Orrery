final RegExp _projectPattern = RegExp(r'^[a-z0-9][a-z0-9-]{0,62}$');
final RegExp _valuePattern = RegExp(r'^[A-Za-z0-9:_.\-]{1,256}$');

class PushConfig {
  const PushConfig({
    required this.apiKey,
    required this.appId,
    required this.senderId,
    required this.projectId,
  });

  final String apiKey;
  final String appId;
  final String senderId;
  final String projectId;

  static PushConfig? tryParse(Map<String, dynamic> json) {
    final key = json['apiKey'];
    final app = json['appId'];
    final sender = json['senderId'];
    final project = json['projectId'];
    if (key is! String || app is! String) return null;
    if (sender is! String || project is! String) return null;
    if (!_valuePattern.hasMatch(key) || !_valuePattern.hasMatch(app)) {
      return null;
    }
    if (!_valuePattern.hasMatch(sender) || !_projectPattern.hasMatch(project)) {
      return null;
    }
    return PushConfig(
      apiKey: key,
      appId: app,
      senderId: sender,
      projectId: project,
    );
  }

  Map<String, dynamic> toJson() => {
    'apiKey': apiKey,
    'appId': appId,
    'senderId': senderId,
    'projectId': projectId,
  };

  @override
  bool operator ==(Object other) =>
      other is PushConfig &&
      other.apiKey == apiKey &&
      other.appId == appId &&
      other.senderId == senderId &&
      other.projectId == projectId;

  @override
  int get hashCode => Object.hash(apiKey, appId, senderId, projectId);
}
