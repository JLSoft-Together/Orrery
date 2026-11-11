import 'dart:io';

import 'package:flutter/scheduler.dart';

class FrameStats {
  final List<FrameTiming> _frames = [];
  late final TimingsCallback _callback = _frames.addAll;
  final Stopwatch _clock = Stopwatch();

  void start() {
    _frames.clear();
    _clock.reset();
    _clock.start();
    SchedulerBinding.instance.addTimingsCallback(_callback);
  }

  FrameReport stop() {
    _clock.stop();
    SchedulerBinding.instance.removeTimingsCallback(_callback);
    return FrameReport.from(List.of(_frames), _clock.elapsed);
  }
}

class FrameReport {
  const FrameReport({
    required this.frames,
    required this.seconds,
    required this.avgBuildMs,
    required this.avgRasterMs,
    required this.worstMs,
    required this.slowFrames,
  });

  final int frames;
  final double seconds;
  final double avgBuildMs;
  final double avgRasterMs;
  final double worstMs;
  final int slowFrames;

  double get fps => seconds == 0 ? 0 : frames / seconds;

  factory FrameReport.from(List<FrameTiming> frames, Duration elapsed) {
    if (frames.isEmpty) {
      return FrameReport(
        frames: 0,
        seconds: elapsed.inMicroseconds / 1e6,
        avgBuildMs: 0,
        avgRasterMs: 0,
        worstMs: 0,
        slowFrames: 0,
      );
    }
    double build = 0;
    double raster = 0;
    double worst = 0;
    int slow = 0;
    for (final f in frames) {
      final b = f.buildDuration.inMicroseconds / 1000;
      final r = f.rasterDuration.inMicroseconds / 1000;
      build += b;
      raster += r;
      final total = f.totalSpan.inMicroseconds / 1000;
      if (total > worst) worst = total;
      if (total > 16.7) slow++;
    }
    return FrameReport(
      frames: frames.length,
      seconds: elapsed.inMicroseconds / 1e6,
      avgBuildMs: build / frames.length,
      avgRasterMs: raster / frames.length,
      worstMs: worst,
      slowFrames: slow,
    );
  }

  @override
  String toString() =>
      'frames=$frames fps=${fps.toStringAsFixed(1)} build=${avgBuildMs.toStringAsFixed(1)}ms '
      'raster=${avgRasterMs.toStringAsFixed(1)}ms worst=${worstMs.toStringAsFixed(1)}ms slow=$slowFrames';
}

double rssMb() => ProcessInfo.currentRss / (1024 * 1024);
