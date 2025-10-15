use std::fs;

use tracing::info;

use crate::{code_gen::flutter::GenFlutter, utils::errors::ResultWithError};

impl GenFlutter {
    pub fn generate_helpers(&self) -> ResultWithError<()> {
        let file = self.out_dir.join("helpers.dart");

        let content = r#"// GENERATED FILE - DO NOT EDIT
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:path/path.dart' as p;
import 'package:screenshot/screenshot.dart';
import 'package:image/image.dart' as img;
import 'package:window_size/window_size.dart';

const updateScreenshots = bool.fromEnvironment('UPDATE_SCREENSHOTS');

/// Custom extensions for WidgetTester and Finders used by generated tests.
extension WidgetTesterExtensions on WidgetTester {
  Future<void> setTestResolution({
    Size size = const Size(1280, 800),
    double ratio = 1.0,
  }) async {
    view.devicePixelRatio = ratio;
    final screens = await getScreenList();
    final screen = screens.first;
    final x = screen.frame.left + (screen.frame.width - size.width) / 2;
    final y = screen.frame.top + (screen.frame.height - size.height) / 2;
    setWindowFrame(Rect.fromLTWH(x, y, size.width, size.height));
    await pumpAndSettle();
  }

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

  Future<void> pumpUntilProgressCompleted(
    Finder finder, {
    Duration timeout = const Duration(seconds: 10),
    Duration step = const Duration(milliseconds: 100),
  }) async {
    var endTime = DateTime.now().add(timeout);
    final lastValueByWidget = <Widget, double>{};
    while (DateTime.now().isBefore(endTime)) {
      await pump(step);
      final progressWidgets = widgetList(finder);
      if (progressWidgets.isEmpty) {
        // No progress widget found, consider it completed
        return;
      }
      bool allCompleted = true;
      for (final widget in progressWidgets) {
        if (widget is LinearProgressIndicator) {
          final value = widget.value;

          if (value != null && value < 1.0) {
            if (lastValueByWidget.containsKey(widget) &&
                lastValueByWidget[widget]! != value) {
              // Progress value changed, reset timeout
              endTime = DateTime.now().add(timeout);
            }

            allCompleted = false;
            lastValueByWidget[widget] = value;
            break;
          }
        } else if (widget is CircularProgressIndicator) {
          final value = widget.value;

          if (value != null && value < 1.0) {
            if (lastValueByWidget.containsKey(widget) &&
                lastValueByWidget[widget]! != value) {
              // Progress value changed, reset timeout
              endTime = DateTime.now().add(timeout);
            }

            allCompleted = false;
            lastValueByWidget[widget] = value;
            break;
          }
        } else {
          throw Exception('Unsupported progress widget: ${widget.runtimeType}');
        }
      }
      if (allCompleted) return;
    }
    throw Exception(
      'Progress not completed within ${timeout.inSeconds}s: $finder',
    );
  }

    Future<void> compareScreenshot(
    String folderName,
    String name, {
    bool update = false,
  }) async {
    // --- Paths ---
    final String projectRoot = Directory.current.path;
    final String folderPath = p.join(
      projectRoot,
      'integration_test',
      'screenshots',
      folderName,
    );
    final String imagePath = p.join(folderPath, '$name.png');

    // --- Capture current screen ---
    final el = find.byWidgetPredicate((w) => w is Screenshot);
    final widget = el.first.evaluate().first.widget as Screenshot;
    final res = await widget.controller.capture(
      delay: const Duration(milliseconds: 10),
    );

    if (res == null) {
      throw Exception('Failed to capture screenshot');
    }

    final File imageFile = File(imagePath);
    await Directory(folderPath).create(recursive: true);

    if (update || !await imageFile.exists() || updateScreenshots) {
      // ✅ Update mode → save new reference image
      await imageFile.writeAsBytes(res);
      debugPrint('Updated reference screenshot: $imagePath');
    } else {
      // 🔍 Compare mode → diff against existing
      final Uint8List existingBytes = await imageFile.readAsBytes();

      final img.Image? oldImg = img.decodeImage(existingBytes);
      final img.Image? newImg = img.decodeImage(res);

      if (oldImg == null || newImg == null) {
        throw Exception('Failed to decode one of the images');
      }

      // Make sure both have same dimensions
      if (oldImg.width != newImg.width || oldImg.height != newImg.height) {
        throw Exception('Screenshot sizes differ for $name');
      }

      // Compute pixel-wise difference
      int diffPixels = 0;
      for (int y = 0; y < oldImg.height; y++) {
        for (int x = 0; x < oldImg.width; x++) {
          if (oldImg.getPixel(x, y) != newImg.getPixel(x, y)) {
            diffPixels++;
          }
        }
      }

      final totalPixels = oldImg.width * oldImg.height;
      final diffRatio = diffPixels / totalPixels;

      // 0.3% threshold
      if (diffRatio > 0.003) {
        final String failedFolderPath = p.join(
          projectRoot,
          'integration_test',
          'screenshots',
          folderName,
          'failed',
        );
        final String failedImagePath = p.join(failedFolderPath, '$name.png');

        final File failedImageFile = File(failedImagePath);
        await Directory(failedFolderPath).create(recursive: true);
        await failedImageFile.writeAsBytes(res);
        debugPrint('Saved failed screenshot: $failedImagePath');

        throw Exception(
          '''Screenshot comparison failed for $name, please update screenshots if the changes are expected.

Please run the following command to update screenshots:
flutter test integration_test --dart-define=UPDATE_SCREENSHOTS=true''',
        );
      }
    }
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
"#;

        fs::write(&file, content)?;
        info!("Generated helpers.dart");
        Ok(())
    }
}
