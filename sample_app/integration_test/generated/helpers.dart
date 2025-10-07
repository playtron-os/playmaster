// GENERATED FILE - DO NOT EDIT
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';

/// Custom extensions for WidgetTester and Finders used by generated tests.
extension WidgetTesterExtensions on WidgetTester {
  /// Pumps until a widget matching [finder] appears, or throws after [timeout].
  Future<void> pumpUntilFound(
    Finder finder, {
    Duration timeout = const Duration(seconds: 5),
    Duration step = const Duration(milliseconds: 100),
  }) async {
    final endTime = DateTime.now().add(timeout);
    while (DateTime.now().isBefore(endTime)) {
      await pump(step);
      if (any(finder)) return;
    }
    throw Exception('Widget not found within ${timeout.inSeconds}s: $finder');
  }

  /// Pumps until a widget matching [finder] disappears.
  Future<void> pumpUntilGone(
    Finder finder, {
    Duration timeout = const Duration(seconds: 5),
    Duration step = const Duration(milliseconds: 100),
  }) async {
    final endTime = DateTime.now().add(timeout);
    while (DateTime.now().isBefore(endTime)) {
      await pump(step);
      if (!any(finder)) return;
    }
    throw Exception(
      'Widget still visible after ${timeout.inSeconds}s: $finder',
    );
  }
}

extension FinderExtensions on CommonFinders {
  /// Finds a [TextField] by its placeholder or label text.
  Finder byPlaceholder(String placeholder) {
    return byWidgetPredicate((w) {
      if (w is TextField && w.decoration?.labelText == placeholder) return true;
      return false;
    }, description: 'TextField(labelText="$placeholder")');
  }

  /// Finds a widget by a [ValueKey] string or prefix.
  Finder byKeyPrefix(String prefix) {
    return byWidgetPredicate((w) {
      final key = w.key;
      if (key is ValueKey<String>) {
        return key.value.startsWith(prefix);
      }
      return false;
    }, description: 'Widget with ValueKey prefix "$prefix"');
  }
}
