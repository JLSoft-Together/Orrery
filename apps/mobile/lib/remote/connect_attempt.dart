import 'link_state.dart';
import 'protocol.dart';

class ConnectAttempt {
  const ConnectAttempt.ready(this.session) : state = null;

  const ConnectAttempt.failed(this.state) : session = null;

  final RemoteSession? session;
  final LinkState? state;
}
