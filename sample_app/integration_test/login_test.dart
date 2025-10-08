import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import 'package:sample_app/main.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('logs in and verifies success', (WidgetTester tester) async {
    // Launch the app
    await tester.pumpWidget(const SampleApp());
    await tester.pumpAndSettle();

    // Enter email and password
    final emailField = find.byKey(const ValueKey('field.email'));
    final passwordField = find.byKey(const ValueKey('field.password'));
    final signInButton = find.byKey(const ValueKey('btn.sign_in'));

    expect(emailField, findsOneWidget);
    expect(passwordField, findsOneWidget);
    expect(signInButton, findsOneWidget);

    await tester.enterText(emailField, 'qa@test.com');
    await tester.enterText(passwordField, 'password123');

    // Tap Sign In
    await tester.tap(signInButton);
    await tester.pumpAndSettle();

    // Verify welcome text is shown
    expect(find.byKey(const ValueKey('welcome_text')), findsOneWidget);
    expect(find.text('Welcome'), findsOneWidget);
  });
}
