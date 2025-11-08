// GENERATED FILE - DO NOT EDIT
import 'dart:async';
import 'dart:io';

import 'package:dbus/dbus.dart';
import 'package:flutter_test/flutter_test.dart';

class E2EDBusObject extends DBusObject {
  E2EDBusObject() : super(DBusObjectPath(PATH));

  static const String PATH = '/one/playmaster/E2E';
  static const String INTERFACE = 'one.playmaster.E2E';

  static const String METHOD_CONTINUE = 'Continue';

  Completer<String> _completer = Completer();

  Future<String> waitForContinue() {
    _completer = Completer<String>();
    return _completer.future;
  }

  DBusMethodResponse continueTest(DBusMethodCall methodCall) {
    if (methodCall.signature != DBusSignature('s')) {
      return DBusMethodErrorResponse.invalidArgs();
    }

    final input = methodCall.values[0].asString();
    if (!_completer.isCompleted) _completer.complete(input);

    return DBusMethodSuccessResponse();
  }

  @override
  Future<DBusMethodResponse> handleMethodCall(DBusMethodCall methodCall) async {
    if (methodCall.interface == INTERFACE) {
      if (methodCall.name == METHOD_CONTINUE) {
        return continueTest(methodCall);
      } else {
        return DBusMethodErrorResponse.unknownMethod();
      }
    } else {
      return DBusMethodErrorResponse.unknownInterface();
    }
  }

  @override
  List<DBusIntrospectInterface> introspect() {
    return [
      DBusIntrospectInterface(
        INTERFACE,
        methods: [DBusIntrospectMethod(METHOD_CONTINUE)],
      ),
    ];
  }
}

extension WidgetTesterExtensions on WidgetTester {
  Future<String> waitForInputViaDBus(String name) async {
    stdout.writeln('Starting DBus listener...');

    final bus = DBusClient.session();

    await bus.requestName(E2EDBusObject.INTERFACE);

    final obj = E2EDBusObject();
    await bus.registerObject(obj);

    stdout
      ..writeln(
        'Waiting for DBus method call ${E2EDBusObject.INTERFACE}.${E2EDBusObject.METHOD_CONTINUE}($name) ...',
      )
      ..writeln(
        'Example: busctl --user call ${E2EDBusObject.INTERFACE} ${E2EDBusObject.PATH} ${E2EDBusObject.INTERFACE} ${E2EDBusObject.METHOD_CONTINUE} s "your input here"',
      );

    final message = await obj.waitForContinue();

    stdout.writeln('Received ${E2EDBusObject.METHOD_CONTINUE}("$message")');
    await bus.close();
    return message;
  }
}
