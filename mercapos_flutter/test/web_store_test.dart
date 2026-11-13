import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_secure_storage_platform_interface/flutter_secure_storage_platform_interface.dart';
import 'package:flutter_secure_storage_web/flutter_secure_storage_web.dart';
import 'package:mercapos_flutter/local_store.dart';
import 'package:mercapos_flutter/models.dart';
import 'package:mercapos_flutter/pos_controller.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'la cola web sobrevive a reabrir IndexedDB y aplica el resultado',
    () async {
      if (!kIsWeb) return;
      FlutterSecureStoragePlatform.instance = FlutterSecureStorageWeb();
      final suffix = DateTime.now().microsecondsSinceEpoch.toString();
      final puesto = 'puesto:test:$suffix';
      final usuario = 'usuario:test:$suffix';
      final id = 'operacion:test:$suffix';
      final producto = Producto(
        id: 'producto:test:$suffix',
        puestoId: puesto,
        nombre: 'Tomate',
        categoria: 'verduras',
        unidad: 'libra',
        precio: 1.25,
        activo: true,
      );
      final secureKey = 'prueba:$suffix';
      await LocalStore.secure.write(key: secureKey, value: 'ok');
      expect(await LocalStore.secure.read(key: secureKey), 'ok');
      await LocalStore.secure.delete(key: secureKey);
      final primero = await LocalStore.open();
      await primero.guardarProductos(puesto, [producto]);
      await primero.guardarVenta({
        'id': id,
        'puesto_id': puesto,
        'usuario_id': usuario,
        'fecha_local': '2026-11-01T10:00:00-06:00',
        'renglones': [
          {'producto_id': producto.id, 'cantidad': 1, 'precio_unitario': 1.25},
        ],
      });
      await primero.db.close();

      final reabierto = await LocalStore.open();
      expect((await reabierto.productos(puesto)).single.nombre, 'Tomate');
      final pendientes = await reabierto.pendientes(puesto, usuario);
      expect(pendientes.single.id, id);
      await reabierto.aplicarResultados(puesto, usuario, pendientes, [
        {
          'operacion_id': id,
          'estado': 'aceptada',
          'mensaje': 'OK',
          'incidencia_id': null,
        },
      ]);
      expect(await reabierto.pendientes(puesto, usuario), isEmpty);
      expect(
        (await reabierto.ventas(puesto, usuario)).single.estado,
        'aceptada',
      );
      await reabierto.db.close();
    },
  );

  test('un 401 de login muestra credenciales inválidas', () async {
    if (!kIsWeb) return;
    final store = await LocalStore.open();
    final controller = PosController(
      store,
      client: MockClient((request) async {
        expect(jsonDecode(request.body)['usuario_id'], 'usuario:admin');
        return http.Response('{"error":"Credenciales inválidas"}', 401);
      }),
    );
    await expectLater(
      controller.login('http://127.0.0.1:3000', 'admin', 'erronea'),
      throwsA(
        isA<ApiError>().having(
          (e) => e.message,
          'mensaje',
          'Credenciales inválidas',
        ),
      ),
    );
    await store.db.close();
  });
}
