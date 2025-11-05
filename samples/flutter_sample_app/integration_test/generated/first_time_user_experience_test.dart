// GENERATED FILE - DO NOT EDIT
import 'dart:ui';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:integration_test/integration_test.dart';
import 'helpers.dart';
import 'vars.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  const invalidPassword = 'wrongpassword';
  const validPassword = 'password123';

  group('First Time User Experience', () {
    testWidgets('Successful Login', (tester) async {
      await tester.initializeTest('');

      //
      await tester.pumpUntilFound(
        find.text('Login'),
        timeout: Duration(milliseconds: 5000),
      );

      //
      await tester.pumpAndSettle();
      await tester.tap(
        find.byPlaceholder('Email'),
        kind: PointerDeviceKind.mouse,
      );
      await tester.pumpAndSettle();

      //
      await tester.pumpAndSettle();
      await tester.tap(
        find.byPlaceholder('Email'),
        kind: PointerDeviceKind.mouse,
      );
      await tester.enterText(
        find.byPlaceholder('Email'),
        '${Common.validEmail}',
      );
      await tester.pumpAndSettle();

      //
      await tester.pumpAndSettle();
      await tester.tap(
        find.byPlaceholder('Password'),
        kind: PointerDeviceKind.mouse,
      );
      await tester.enterText(
        find.byPlaceholder('Password'),
        '${validPassword}',
      );
      await tester.pumpAndSettle();

      //
      await tester.pumpAndSettle();
      await tester.tap(find.text('Sign In'), kind: PointerDeviceKind.mouse);
      await tester.pumpAndSettle();

      //
      await tester.pumpUntilProgressCompleted(
        find.byType(LinearProgressIndicator),
        timeout: Duration(milliseconds: 30000),
      );

      //
      await tester.pumpUntilFound(
        find.text('Welcome'),
        timeout: Duration(milliseconds: 5000),
      );

      //
      await tester.compareScreenshot(
        'first_time_user_experience_test',
        'screenshot_welcome',
      );
    });

    testWidgets('Invalid Login', (tester) async {
      await tester.initializeTest('');

      //
      await tester.pumpUntilFound(
        find.text('Login'),
        timeout: Duration(milliseconds: 5000),
      );

      //
      await tester.pumpAndSettle();
      await tester.tap(
        find.byPlaceholder('Email'),
        kind: PointerDeviceKind.mouse,
      );
      await tester.pumpAndSettle();

      //
      await tester.pumpAndSettle();
      await tester.tap(
        find.byPlaceholder('Email'),
        kind: PointerDeviceKind.mouse,
      );
      await tester.enterText(
        find.byPlaceholder('Email'),
        '${Common.validEmail}',
      );
      await tester.pumpAndSettle();

      //
      await tester.pumpAndSettle();
      await tester.tap(
        find.byPlaceholder('Password'),
        kind: PointerDeviceKind.mouse,
      );
      await tester.enterText(
        find.byPlaceholder('Password'),
        '${invalidPassword}',
      );
      await tester.pumpAndSettle();

      //
      await tester.pumpAndSettle();
      await tester.tap(find.text('Sign In'), kind: PointerDeviceKind.mouse);
      await tester.pumpAndSettle();

      //
      await tester.pump(Duration(milliseconds: 1000));

      //
      expect(find.text('Invalid credentials'), findsOneWidget);
    });
  });
}
