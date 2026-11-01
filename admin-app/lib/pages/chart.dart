// A small line chart (no chart package): time on x, value on y, a few series.
import 'dart:math' as math;

import 'package:flutter/material.dart';

class MetricChart extends StatelessWidget {
  const MetricChart({super.key, required this.title, required this.series});
  final String title;
  final Map<String, List<(int, double)>> series;

  static const colors = [Colors.green, Colors.orange, Colors.red, Colors.blue];

  @override
  Widget build(BuildContext context) {
    final all = series.values.expand((s) => s).toList();
    final maxV = all.isEmpty ? 1.0 : all.map((p) => p.$2).reduce(math.max);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '$title (max ${maxV.toStringAsFixed(maxV < 10 ? 1 : 0)})',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            SizedBox(
              height: 140,
              child: CustomPaint(
                size: Size.infinite,
                painter: _Painter(series),
              ),
            ),
            Wrap(
              spacing: 12,
              children: [
                for (final (i, k) in series.keys.indexed)
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Container(
                        width: 10,
                        height: 10,
                        color: colors[i % colors.length],
                      ),
                      const SizedBox(width: 4),
                      Text(k),
                    ],
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _Painter extends CustomPainter {
  _Painter(this.series);
  final Map<String, List<(int, double)>> series;

  @override
  void paint(Canvas canvas, Size size) {
    final all = series.values.expand((s) => s).toList();
    if (all.isEmpty) return;
    final minT = all.map((p) => p.$1).reduce(math.min).toDouble();
    final maxT = all.map((p) => p.$1).reduce(math.max).toDouble();
    final maxV = math.max(1.0, all.map((p) => p.$2).reduce(math.max));
    final grid = Paint()
      ..color = Colors.grey.withValues(alpha: 0.3)
      ..strokeWidth = 1;
    for (var i = 0; i <= 4; i++) {
      final y = size.height * i / 4;
      canvas.drawLine(Offset(0, y), Offset(size.width, y), grid);
    }
    for (final (i, s) in series.values.indexed) {
      if (s.isEmpty) continue;
      final p = Paint()
        ..color = MetricChart.colors[i % MetricChart.colors.length]
        ..strokeWidth = 2
        ..style = PaintingStyle.stroke;
      final path = Path();
      for (final (j, pt) in s.indexed) {
        final x = maxT == minT
            ? size.width / 2
            : (pt.$1 - minT) / (maxT - minT) * size.width;
        final y = size.height - pt.$2 / maxV * size.height;
        j == 0 ? path.moveTo(x, y) : path.lineTo(x, y);
      }
      canvas.drawPath(path, p);
    }
  }

  @override
  bool shouldRepaint(covariant _Painter old) => old.series != series;
}
