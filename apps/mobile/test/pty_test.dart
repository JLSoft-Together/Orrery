import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:clock/clock.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/connection.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/pty_stream.dart';

import 'connection_test.dart' show settle;
import 'support/fakes.dart';

class Bed {
  Bed(this.async) : net = FakeNet()..now = (() => clock.now()), servers = [] {
    deps = settle(async, testDeps(net: net, machines: [testMachine()]));
    net.lan = (_) {
      final server = FakeServer();
      servers.add(server);
      return server;
    };
    conn = MachineConnection(machineId: 'm1', deps: deps);
    pty = PtyStreams(conn);
    unawaited(conn.connect());
    async.elapse(const Duration(milliseconds: 20));
  }

  final FakeAsync async;
  final FakeNet net;
  final List<FakeServer> servers;
  late final ConnectionDeps deps;
  late final MachineConnection conn;
  late final PtyStreams pty;

  PtyAttachment attach(String session, {String? pane}) {
    PtyAttachment? result;
    pty.attach(session, 80, 24, paneKey: pane).then((a) => result = a);
    async.flushMicrotasks();
    return result!;
  }

  List<Map<String, dynamic>> get commands => servers.last.commands;
}

void main() {
  test('attach sends pty.attach and routes output by stream id', () {
    fakeAsync((async) {
      final bed = Bed(async);
      final a = bed.attach('s1');
      final b = bed.attach('s2');
      final got = <String>[];
      a.output.listen((bytes) => got.add('a:${utf8.decode(bytes)}'));
      b.output.listen((bytes) => got.add('b:${utf8.decode(bytes)}'));
      final attaches = bed.commands.where((c) => c['t'] == 'pty.attach');
      expect(attaches.map((c) => c['stream']), [1, 2]);
      expect(attaches.first['sessionId'], 's1');
      expect(attaches.first['cols'], 80);
      bed.servers.last.pushPty(2, utf8.encode('two'));
      bed.servers.last.pushPty(1, utf8.encode('one'));
      bed.servers.last.pushPty(9, utf8.encode('stray'));
      async.flushMicrotasks();
      expect(got, ['b:two', 'a:one']);
    });
  });

  test('at most 4 attachments, the fifth is refused with limit', () {
    fakeAsync((async) {
      final bed = Bed(async);
      for (var i = 0; i < 4; i++) {
        bed.attach('s$i');
      }
      Object? error;
      bed.pty
          .attach('s5', 80, 24)
          .then(
            (_) {},
            onError: (Object e) {
              error = e;
            },
          );
      async.flushMicrotasks();
      expect((error as RemoteCommandException).code, 'limit');
      expect(bed.pty.attachments, hasLength(4));
    });
  });

  test('input goes out as kind 2 and is coalesced inside 30 ms', () {
    fakeAsync((async) {
      final bed = Bed(async);
      final a = bed.attach('s1');
      final echoed = <String>[];
      a.output.listen((b) => echoed.add(utf8.decode(b)));
      bed.pty.input(a, Uint8List.fromList(utf8.encode('l')));
      bed.pty.input(a, Uint8List.fromList(utf8.encode('s')));
      bed.pty.input(a, Uint8List.fromList(utf8.encode(' -')));
      async.flushMicrotasks();
      expect(bed.servers.last.ptyInput.map(utf8.decode), ['l']);
      async.elapse(const Duration(milliseconds: 35));
      expect(bed.servers.last.ptyInput.map(utf8.decode), ['l', 's -']);
      async.elapse(const Duration(milliseconds: 100));
      bed.pty.input(a, Uint8List.fromList(utf8.encode('x')));
      expect(bed.servers.last.ptyInput.map(utf8.decode), ['l', 's -', 'x']);
      expect(echoed.first, 'echo:l');
    });
  });

  test('resize and detach send commands, detach frees the slot', () {
    fakeAsync((async) {
      final bed = Bed(async);
      final a = bed.attach('s1');
      unawaited(bed.pty.resize(a, 100, 30));
      async.flushMicrotasks();
      final resize = bed.commands.firstWhere((c) => c['t'] == 'pty.resize');
      expect([resize['stream'], resize['cols'], resize['rows']], [1, 100, 30]);
      unawaited(bed.pty.detach(a));
      async.flushMicrotasks();
      expect(bed.commands.any((c) => c['t'] == 'pty.detach'), isTrue);
      expect(bed.pty.attachments, isEmpty);
    });
  });

  test('background detaches every pane, foreground attaches them again', () {
    fakeAsync((async) {
      final bed = Bed(async);
      bed.attach('s1');
      bed.attach('s2');
      unawaited(bed.pty.suspendAll());
      async.flushMicrotasks();
      expect(bed.commands.where((c) => c['t'] == 'pty.detach'), hasLength(2));
      unawaited(bed.pty.resumeAll());
      async.flushMicrotasks();
      expect(bed.commands.where((c) => c['t'] == 'pty.attach'), hasLength(4));
    });
  });

  test(
    'after a reconnect every attachment is attached again on the new session',
    () {
      fakeAsync((async) {
        final bed = Bed(async);
        bed.attach('s1');
        bed.servers.last.serverClose(null);
        async.elapse(const Duration(seconds: 2));
        expect(bed.servers, hasLength(2));
        final attaches = bed.servers.last.commands
            .where((c) => c['t'] == 'pty.attach')
            .toList();
        expect(attaches, hasLength(1));
        expect(attaches.single['stream'], 1);
        expect(attaches.single['sessionId'], 's1');
      });
    },
  );

  test('pty.ready clears the tail, overflow re-attaches, tail is capped', () {
    fakeAsync((async) {
      final bed = Bed(async);
      final a = bed.attach('s1', pane: 'paneA');
      final events = <String>[];
      a.events.listen((e) => events.add(e['t'] as String));
      bed.servers.last.pushPty(1, Uint8List(ptyTailBytes - 10));
      bed.servers.last.pushPty(1, Uint8List.fromList(List.filled(100, 65)));
      async.flushMicrotasks();
      expect(a.tail.length, ptyTailBytes);
      expect(a.tail.last, 65);
      expect(bed.pty.tails['paneA']!.length, ptyTailBytes);
      bed.servers.last.pushJson({
        't': 'pty.ready',
        'stream': 1,
        'cols': 80,
        'rows': 24,
      });
      async.flushMicrotasks();
      expect(a.tail, isEmpty);
      final before = bed.commands.where((c) => c['t'] == 'pty.attach').length;
      bed.servers.last.pushJson({'t': 'pty.overflow', 'stream': 1});
      async.flushMicrotasks();
      expect(events, ['pty.ready', 'pty.overflow']);
      expect(
        bed.commands.where((c) => c['t'] == 'pty.attach').length,
        before + 1,
      );
    });
  });

  test('input while not connected is dropped, not queued', () {
    fakeAsync((async) {
      final bed = Bed(async);
      final a = bed.attach('s1');
      unawaited(bed.conn.disconnect());
      async.flushMicrotasks();
      bed.pty.input(a, Uint8List.fromList([1]));
      async.elapse(const Duration(seconds: 1));
      expect(bed.servers.last.ptyInput, isEmpty);
    });
  });
}
