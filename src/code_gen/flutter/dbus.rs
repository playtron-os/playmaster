use std::fs;

use tracing::info;

use crate::{
    code_gen::flutter::GenFlutter,
    utils::{dbus::DbusUtils, errors::EmptyResult},
};

impl GenFlutter {
    pub fn generate_dbus(&self) -> EmptyResult {
        let file = self.out_dir.join("dbus.dart");

        let content = r#"// GENERATED FILE - DO NOT EDIT
import 'dart:async';
import 'dart:io';

import 'package:dbus/dbus.dart';
import 'package:flutter_test/flutter_test.dart';

class E2EDBusObject extends DBusObject {
  E2EDBusObject() : super(DBusObjectPath(PATH));

  static const String PATH = '{DBUS_PATH}';
  static const String INTERFACE = '{DBUS_INTERFACE}';

  static const String METHOD_CONTINUE = '{DBUS_METHOD_CONTINUE}';

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
      DBusIntrospectInterface(INTERFACE, methods: [
        DBusIntrospectMethod(METHOD_CONTINUE),
      ]),
    ];
  }
}

extension WidgetTesterExtensions on WidgetTester {
  Future<String> waitForInputViaDBus(String name) async {
    stdout.writeln('Starting DBus listener for input "$name"...');

    final bus = DBusClient.session();

    await bus.requestName(E2EDBusObject.INTERFACE);

    final obj = E2EDBusObject();
    await bus.registerObject(obj);

    stdout
      ..writeln('Waiting for DBus method call ${E2EDBusObject.INTERFACE}.${E2EDBusObject.METHOD_CONTINUE}(...) ...')
      ..writeln(
        'Example: busctl --user call ${E2EDBusObject.INTERFACE} ${E2EDBusObject.PATH} ${E2EDBusObject.INTERFACE} ${E2EDBusObject.METHOD_CONTINUE} s "your input here"',
      );

    final message = await obj.waitForContinue();

    stdout.writeln('Received ${E2EDBusObject.METHOD_CONTINUE}("$message")');
    await bus.close();
    return message;
  }
}
"#;

        fs::write(
            &file,
            content
                .replace("{DBUS_PATH}", DbusUtils::get_dbus_path())
                .replace("{DBUS_INTERFACE}", DbusUtils::get_dbus_interface())
                .replace(
                    "{DBUS_METHOD_CONTINUE}",
                    DbusUtils::get_dbus_method_continue(),
                ),
        )?;
        info!("Generated dbus.dart");
        Ok(())
    }
}
