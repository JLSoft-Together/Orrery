import 'chat_protocol.dart';
import 'protocol.dart';
import 'store.dart';

extension ChatViewX on MachineView {
  bool get chatEnabled {
    final value = hello?['chat'];
    return value is num && value >= ChatLimits.proto;
  }
}

Map<String, ChatLast>? nextChatLast(
  Map<String, ChatLast> current,
  JsonMap event,
) {
  final parsed = ChatEvent.parse(event);
  if (parsed is! ChatLastEvent) return null;
  final next = parsed.full ? <String, ChatLast>{} : {...current};
  for (final item in parsed.items) {
    if (item.text == null) {
      next.remove(item.paneKey);
    } else {
      next[item.paneKey] = item;
    }
  }
  return next;
}
