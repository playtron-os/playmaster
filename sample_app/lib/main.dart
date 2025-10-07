import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:path_provider/path_provider.dart';
import 'package:screenshot/screenshot.dart';

void main() {
  runApp(
    Screenshot(controller: ScreenshotController(), child: const SampleApp()),
  );
}

class SampleApp extends StatelessWidget {
  const SampleApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(title: 'Sample Test App', home: const LoginScreen());
  }
}

class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final GlobalKey _repaintKey = GlobalKey();
  final TextEditingController _emailController = TextEditingController();
  final TextEditingController _passwordController = TextEditingController();
  bool _loggedIn = false;

  void _login() {
    if (_emailController.text == "qa@playtron.one" &&
        _passwordController.text == "password123") {
      setState(() => _loggedIn = true);
    } else {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text("Invalid credentials")));
    }
  }

  Future<void> _capturePng() async {
    try {
      // Get render object from key
      RenderRepaintBoundary boundary =
          _repaintKey.currentContext!.findRenderObject()
              as RenderRepaintBoundary;

      // Convert to image
      ui.Image image = await boundary.toImage(pixelRatio: 3.0);

      // Encode as PNG
      ByteData? byteData = await image.toByteData(
        format: ui.ImageByteFormat.png,
      );
      Uint8List pngBytes = byteData!.buffer.asUint8List();

      // Save to file
      final dir = await getApplicationDocumentsDirectory();
      final file = File('${dir.path}/screenshot.png');
      await file.writeAsBytes(pngBytes);

      debugPrint('✅ Screenshot saved to: ${file.path}');
    } catch (e) {
      debugPrint('❌ Failed to capture screenshot: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return RepaintBoundary(
      key: _repaintKey,
      child: Scaffold(
        appBar: AppBar(title: const Text("Login")),
        body: Center(
          child:
              _loggedIn
                  ? Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    spacing: 20,
                    children: [
                      const Text("Welcome", key: ValueKey("welcome_text")),
                      ElevatedButton(
                        onPressed: _capturePng,
                        child: const Text('Capture Screenshot'),
                      ),
                    ],
                  )
                  : Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      TextField(
                        key: const ValueKey("field.email"),
                        controller: _emailController,
                        decoration: const InputDecoration(labelText: "Email"),
                      ),
                      TextField(
                        key: const ValueKey("field.password"),
                        controller: _passwordController,
                        decoration: const InputDecoration(
                          labelText: "Password",
                        ),
                        obscureText: true,
                      ),
                      ElevatedButton(
                        key: const ValueKey("btn.sign_in"),
                        onPressed: _login,
                        child: const Text("Sign In"),
                      ),
                    ],
                  ),
        ),
      ),
    );
  }
}
