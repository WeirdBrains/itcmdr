import 'dart:math' as math;

import 'package:flutter/material.dart';

import 'proto/itcmdr.pb.dart';

/// One bar per bucket: `ts` (unix millis) -> value.
class ChartBar {
  final int ts;
  final double value;
  const ChartBar(this.ts, this.value);
}

/// Combine per-machine points with [prefix] into one series per bucket.
/// [average] = mean across machines (uptime), otherwise the sum (incidents).
List<ChartBar> seriesByBucket(
  Iterable<MetricPoint> points,
  String prefix, {
  bool average = false,
}) {
  final sum = <int, double>{};
  final n = <int, int>{};
  for (final p in points.where((p) => p.key.startsWith('$prefix:'))) {
    final ts = p.ts.toInt();
    sum[ts] = (sum[ts] ?? 0) + p.value;
    n[ts] = (n[ts] ?? 0) + 1;
  }
  final keys = sum.keys.toList()..sort();
  return [
    for (final k in keys) ChartBar(k, average ? sum[k]! / n[k]! : sum[k]!),
  ];
}

String _label(int ts) {
  final d = DateTime.fromMillisecondsSinceEpoch(ts, isUtc: true);
  return '${d.month}/${d.day}';
}

/// Simple bar chart (no extra packages). [maxY] fixes the scale (e.g. 1.0 for ratios).
class BarChart extends StatelessWidget {
  final String title;
  final List<ChartBar> bars;
  final double? maxY;
  final String Function(double)? format;
  const BarChart({
    super.key,
    required this.title,
    required this.bars,
    this.maxY,
    this.format,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final fmt = format ?? (v) => v.toStringAsFixed(0);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            if (bars.isEmpty) const Text('No data'),
            if (bars.isNotEmpty)
              SizedBox(
                height: 180,
                child: CustomPaint(
                  size: Size.infinite,
                  painter: _BarPainter(
                    bars,
                    maxY,
                    scheme.primary,
                    scheme.onSurface,
                    fmt,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _BarPainter extends CustomPainter {
  final List<ChartBar> bars;
  final double? maxY;
  final Color bar;
  final Color text;
  final String Function(double) fmt;
  _BarPainter(this.bars, this.maxY, this.bar, this.text, this.fmt);

  void _text(Canvas c, String s, Offset at, double maxW) {
    final tp = TextPainter(
      text: TextSpan(
        style: TextStyle(fontSize: 10, color: text),
        text: s,
      ),
      textDirection: TextDirection.ltr,
      maxLines: 1,
      ellipsis: '',
    )..layout(maxWidth: math.max(maxW, 1));
    tp.paint(c, at - Offset(tp.width / 2, 0));
  }

  @override
  void paint(Canvas canvas, Size size) {
    const labelH = 14.0;
    final top = maxY ?? bars.map((b) => b.value).fold<double>(1, math.max);
    final slot = size.width / bars.length;
    final h = size.height - 2 * labelH;
    final paint = Paint()..color = bar;
    final every = math.max(1, (40 / slot).ceil());
    for (var i = 0; i < bars.length; i++) {
      final v = bars[i].value.clamp(0, top).toDouble();
      final bh = top <= 0 ? 0.0 : h * v / top;
      final x = i * slot;
      canvas.drawRect(
        Rect.fromLTWH(x + slot * 0.15, labelH + h - bh, slot * 0.7, bh),
        paint,
      );
      if (i % every == 0) {
        _text(
          canvas,
          fmt(bars[i].value),
          Offset(x + slot / 2, labelH + h - bh - labelH),
          slot * every,
        );
        _text(
          canvas,
          _label(bars[i].ts),
          Offset(x + slot / 2, size.height - labelH),
          slot * every,
        );
      }
    }
  }

  @override
  bool shouldRepaint(_BarPainter old) => old.bars != bars || old.maxY != maxY;
}
