import 'package:app/backup_page.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('last deep check: never, ok, due and failed', () {
    final now = DateTime(2026, 11, 2);
    int ms(int days) =>
        now.subtract(Duration(days: days)).millisecondsSinceEpoch;
    expect(deepCheckText(0, 'none', now).warn, isTrue);
    expect(deepCheckText(0, 'none', now).text, contains('never'));
    final ok = deepCheckText(ms(10), 'ok', now);
    expect((ok.text, ok.warn), ('Last deep check: 10 days ago (ok).', false));
    final due = deepCheckText(ms(91), 'ok', now);
    expect(due.warn, isTrue);
    expect(due.text, contains('91 days ago'));
    expect(deepCheckText(ms(91), 'ok', now, maxDays: 120).warn, isFalse);
    expect(deepCheckText(ms(1), 'failed', now).warn, isTrue);
  });
}
