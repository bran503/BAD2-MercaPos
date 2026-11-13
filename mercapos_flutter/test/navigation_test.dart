import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:mercapos_flutter/local_store.dart';
import 'package:mercapos_flutter/main.dart';
import 'package:mercapos_flutter/models.dart';
import 'package:mercapos_flutter/pos_controller.dart';

void main() {
  testWidgets('el administrador puede navegar a las pantallas web', (
    tester,
  ) async {
    if (!kIsWeb) return;
    final store = (await tester.runAsync(LocalStore.open))!;
    final controller = PosController(
      store,
      client: MockClient((request) async => http.Response('[]', 200)),
    );
    controller.usuario = const Usuario(
      id: 'usuario:test',
      nombre: 'Admin',
      rol: 'administrador',
      puestoId: 'puesto:test',
    );
    controller.token = 'test-token';
    await tester.pumpWidget(MercaPosApp(controller: controller));
    expect(find.text('MercaPOS · Vender'), findsOneWidget);

    await tester.tap(find.byIcon(Icons.menu));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Usuarios'));
    await tester.pumpAndSettle();
    expect(find.text('Nuevo usuario'), findsOneWidget);

    await tester.tap(find.byIcon(Icons.menu));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Productos'));
    await tester.pumpAndSettle();
    expect(find.text('Nuevo producto'), findsOneWidget);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.runAsync(store.db.close);
  });
}
