enum LinkStatus {
  idle,
  discovering,
  handshaking,
  ready,
  reconnecting,
  offline,
  protoTooOld,
  revoked,
}

enum LinkRoute { lan, relay }

enum LinkReason {
  none,
  background,
  networkLost,
  desktopAbsent,
  remoteOff,
  restarting,
  tokenExpired,
  rateLimited,
  handshakeFailed,
  noRoute,
}

class LinkState {
  const LinkState({
    this.status = LinkStatus.idle,
    this.route,
    this.reason = LinkReason.none,
    this.attempt = 0,
    this.retryIn,
    this.minProto,
  });

  final LinkStatus status;
  final LinkRoute? route;
  final LinkReason reason;
  final int attempt;
  final Duration? retryIn;
  final int? minProto;

  bool get isTerminal =>
      status == LinkStatus.protoTooOld || status == LinkStatus.revoked;

  LinkState copyWith({
    LinkStatus? status,
    LinkRoute? route,
    LinkReason? reason,
    int? attempt,
    Duration? retryIn,
    int? minProto,
    bool clearRoute = false,
    bool clearRetry = false,
  }) => LinkState(
    status: status ?? this.status,
    route: clearRoute ? null : (route ?? this.route),
    reason: reason ?? this.reason,
    attempt: attempt ?? this.attempt,
    retryIn: clearRetry ? null : (retryIn ?? this.retryIn),
    minProto: minProto ?? this.minProto,
  );

  @override
  bool operator ==(Object other) =>
      other is LinkState &&
      other.status == status &&
      other.route == route &&
      other.reason == reason &&
      other.attempt == attempt &&
      other.retryIn == retryIn &&
      other.minProto == minProto;

  @override
  int get hashCode =>
      Object.hash(status, route, reason, attempt, retryIn, minProto);

  @override
  String toString() =>
      'LinkState($status, $route, $reason, attempt=$attempt, retryIn=$retryIn)';
}
