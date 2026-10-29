import 'package:flutter_test/flutter_test.dart';
import 'package:app/main.dart' as app;

void main() {
  testWidgets('app builds', (tester) async {
    app.main();
    await tester.pumpAndSettle();
  });
}
