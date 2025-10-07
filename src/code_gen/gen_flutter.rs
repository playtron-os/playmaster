use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

use tracing::{error, info};

use crate::{
    code_gen::gen_iface::CodeGenTrait,
    models::{
        args::AppArgs,
        config::{Config, ProjectType},
        feature_test::{self, FeatureTest, Step, WaitFor},
    },
    utils::{
        dir::DirUtils,
        errors::{EmptyResult, ResultWithError},
    },
};

#[allow(dead_code)]
pub struct GenFlutter {
    args: AppArgs,
    config: Config,
    out_dir: PathBuf,
}

impl CodeGenTrait for GenFlutter {
    fn get_type(&self) -> ProjectType {
        ProjectType::Flutter
    }

    fn run(&self, features: &[FeatureTest]) -> EmptyResult {
        self.generate_helpers()?;

        for feature in features {
            feature.generate_dart(&self.out_dir)?;
        }

        self.generate_all_entrypoint(features)?;
        self.run_dart_format();
        self.run_dart_fix();

        Ok(())
    }
}

impl GenFlutter {
    pub fn from_exec_dir(args: AppArgs, config: Config) -> ResultWithError<Self> {
        let cwd = DirUtils::exec_dir()?;
        let out_dir = cwd.join("integration_test/generated");
        fs::create_dir_all(&out_dir)?;

        Ok(Self {
            args,
            config,
            out_dir,
        })
    }

    fn generate_helpers(&self) -> ResultWithError<()> {
        let file = self.out_dir.join("helpers.dart");

        let content = r#"// GENERATED FILE - DO NOT EDIT
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;
import 'package:screenshot/screenshot.dart';
import 'package:image/image.dart' as img;

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

  Future<void> compareScreenshot(String name, {bool update = false}) async {
    // --- Paths ---
    final String projectRoot = Directory.current.path;
    final String folderPath = p.join(
      projectRoot,
      'integration_test',
      'screenshots',
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

    if (update || !await imageFile.exists()) {
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

      // 0.1% threshold
      if (diffRatio > 0.001) {
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

    /// Generate an `all_tests.dart` file that imports and runs all generated tests.
    fn generate_all_entrypoint(&self, features: &[FeatureTest]) -> EmptyResult {
        let entry_file = self.out_dir.join("all_tests.dart");
        let mut content = String::new();

        content.push_str("// GENERATED FILE - DO NOT EDIT\n");
        content.push_str("// This file aggregates all generated integration tests.\n\n");

        // Import each generated test file
        for feature in features {
            let import_name = feature.name.to_lowercase().replace(' ', "_") + "_test.dart";
            let alias = feature
                .name
                .to_lowercase()
                .replace(' ', "_")
                .replace('-', "_");
            content.push_str(&format!("import '{import_name}' as {alias};\n"));
        }

        content.push_str("\nvoid main() {\n");
        for feature in features {
            let alias = feature
                .name
                .to_lowercase()
                .replace(' ', "_")
                .replace('-', "_");
            content.push_str(&format!("  {alias}.main();\n"));
        }
        content.push_str("}\n");

        fs::write(entry_file, content)?;
        info!("Generated all_tests.dart entrypoint");
        Ok(())
    }

    fn run_dart_fix(&self) {
        let path = self.out_dir.clone().to_string_lossy().to_string();
        info!("Running dart fix on {}", path);

        let status = process::Command::new("dart")
            .args(["fix", path.as_str(), "--apply"])
            .status();

        match status {
            Ok(s) if s.success() => info!("✅ Dart fix completed successfully."),
            Ok(s) => error!("❌ Dart fix failed with status: {}", s),
            Err(e) => error!("❌ Failed to run dart fix: {}", e),
        }
    }

    fn run_dart_format(&self) {
        let path = self.out_dir.clone().to_string_lossy().to_string();
        info!("Running dart format on {}", path);

        let status = process::Command::new("dart")
            .args(["format", path.as_str()])
            .status();

        match status {
            Ok(s) if s.success() => info!("✅ Dart format completed successfully."),
            Ok(s) => error!("❌ Dart format failed with status: {}", s),
            Err(e) => error!("❌ Failed to run dart format: {}", e),
        }
    }
}

impl FeatureTest {
    pub fn generate_dart(&self, out_dir: &Path) -> EmptyResult {
        let file_name = self.name.to_lowercase().replace(' ', "_") + "_test.dart";
        let file_path = out_dir.join(file_name);
        let mut out = String::new();

        // Header
        out.push_str("// GENERATED FILE - DO NOT EDIT\n");
        out.push_str("import 'package:flutter_test/flutter_test.dart';\n");
        out.push_str("import 'package:integration_test/integration_test.dart';\n");
        out.push_str("import 'package:sample_app/main.dart' as app;\n\n");
        out.push_str("import 'helpers.dart';\n\n");
        out.push_str("void main() {\n");
        out.push_str("  IntegrationTestWidgetsFlutterBinding.ensureInitialized();\n\n");
        out.push_str(&format!("  group('{}', () {{\n", self.name));

        // Test cases
        for test in &self.tests {
            out.push_str(&format!(
                "    testWidgets('{}', (tester) async {{\n",
                test.name
            ));
            out.push_str("      app.main();\n");
            out.push_str("      await tester.pumpAndSettle();\n\n");

            for step in &test.steps {
                out.push_str(&step.to_dart_code());
            }

            out.push_str("    });\n\n");
        }

        out.push_str("  });\n}\n");

        fs::write(file_path, out)?;
        Ok(())
    }
}

impl Step {
    pub fn to_dart_code(&self) -> String {
        match self {
            Step::WaitFor { wait_for } => match wait_for {
                WaitFor::Text { text } => format!(
                    "      await tester.pumpUntilFound(find.text('{}'));\n",
                    text
                ),
                WaitFor::Delay { delay } => format!(
                    "      await tester.pump(Duration(milliseconds: {}));\n",
                    delay
                ),
            },
            Step::Tap { tap } => match &tap.target {
                feature_test::Target::Placeholder { placeholder } => format!(
                    "      await tester.tap(find.byPlaceholder('{}'));\n",
                    placeholder
                ),
                feature_test::Target::Text { text } => {
                    format!("      await tester.tap(find.text('{}'));\n", text)
                }
            },
            Step::Type { r#type } => match &r#type.by {
                feature_test::Target::Placeholder { placeholder } => format!(
                    "      await tester.enterText(find.byPlaceholder('{}'), '{}');\n",
                    placeholder, r#type.value
                ),
                feature_test::Target::Text { text } => format!(
                    "      await tester.enterText(find.text('{}'), '{}');\n",
                    text, r#type.value
                ),
            },
            Step::Match { r#match } => match &r#match.target {
                feature_test::MatchTarget::Text { text } => {
                    format!("      expect(find.text('{}'), findsOneWidget);\n", text)
                }
                feature_test::MatchTarget::Screenshot { screenshot } => {
                    format!("      await tester.compareScreenshot('{}');\n", screenshot)
                }
            },
        }
    }
}
