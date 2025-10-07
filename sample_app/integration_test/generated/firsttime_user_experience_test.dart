// GENERATED FILE - DO NOT EDIT
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:sample_app/main.dart' as app;

import 'helpers.dart';

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  group('FirstTime User Experience', () {
    testWidgets('Successful Login', (tester) async {
      app.main();
      await tester.pumpAndSettle();

      await tester.pumpUntilFound(find.text('Login'));
      await tester.tap(find.byPlaceholder('Email'));
      await tester.enterText(find.byPlaceholder('Email'), 'qa@playtron.one');
      await tester.enterText(find.byPlaceholder('Password'), 'password123');
      await tester.tap(find.text('Sign In'));
      await tester.pumpUntilFound(find.text('Welcome'));
      expect(find.text('Welcome'), findsOneWidget);
      await binding.takeScreenshot('screenshots/screenshot.png');
    });

    testWidgets('Invalid Login', (tester) async {
      app.main();
      await tester.pumpAndSettle();

      await tester.pumpUntilFound(find.text('Login'));
      await tester.tap(find.byPlaceholder('Email'));
      await tester.enterText(find.byPlaceholder('Email'), 'qa@playtron.one');
      await tester.enterText(find.byPlaceholder('Password'), 'password');
      await tester.tap(find.text('Sign In'));
      expect(find.text('Invalid credentials'), findsOneWidget);
    });
  });
}
