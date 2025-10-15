// GENERATED FILE - DO NOT EDIT
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:integration_test/integration_test.dart';
import 'package:sample_app/main.dart' as app;

import 'helpers.dart';
import 'vars.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  const invalidPassword = 'wrongpassword';
  const validPassword = 'password123';

  group('First Time User Experience', () {
    testWidgets('Successful Login', (tester) async {
      await tester.setTestResolution();

      app.main();
      await tester.pumpAndSettle();

      await tester.pumpUntilFound(find.text('Login'));
      await tester.tap(find.byPlaceholder('Email'));
      await tester.enterText(
        find.byPlaceholder('Email'),
        Common.validEmail,
      );
      await tester.enterText(
        find.byPlaceholder('Password'),
        validPassword,
      );
      await tester.tap(find.text('Sign In'));
      await tester.pumpUntilProgressCompleted(
        find.byType(LinearProgressIndicator),
      );
      await tester.pumpUntilFound(find.text('Welcome'));
      await tester.compareScreenshot(
        'first_time_user_experience_test',
        'screenshot_welcome',
      );
    });

    testWidgets('Invalid Login', (tester) async {
      await tester.setTestResolution();

      app.main();
      await tester.pumpAndSettle();

      await tester.pumpUntilFound(find.text('Login'));
      await tester.tap(find.byPlaceholder('Email'));
      await tester.enterText(
        find.byPlaceholder('Email'),
        Common.validEmail,
      );
      await tester.enterText(
        find.byPlaceholder('Password'),
        invalidPassword,
      );
      await tester.tap(find.text('Sign In'));
      await tester.pump(Duration(milliseconds: 1000));
      expect(find.text('Invalid credentials'), findsOneWidget);
    });
  });
}
