import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage_platform_interface/flutter_secure_storage_platform_interface.dart';
import 'package:flutter_secure_storage_web/flutter_secure_storage_web.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:mercapos_flutter/local_store.dart';
import 'package:mercapos_flutter/pos_controller.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'la web envía revisión, conserva operación y descarta sesión vencida',
    () async {
      if (!kIsWeb) return;
      FlutterSecureStoragePlatform.instance = FlutterSecureStorageWeb();
      final suffix = DateTime.now().microsecondsSinceEpoch.toString();
      final puesto = 'puesto:test:$suffix';
      final user = 'usuario:test:$suffix';
      final product = 'producto:test:$suffix';
      final requests = <http.Request>[];
      var expired = false;
      final store = await LocalStore.open();
      final controller = PosController(
        store,
        client: MockClient((request) async {
          requests.add(request);
          final path = request.url.path;
          final segments = request.url.pathSegments;
          if (path == '/auth/login') {
            return http.Response(
              jsonEncode({
                'token': 'test-token',
                'usuario': {
                  'id': user,
                  'nombre': 'Prueba',
                  'rol': 'administrador',
                  'puesto_id': puesto,
                },
              }),
              200,
            );
          }
          if (path == '/productos' && request.method == 'GET') {
            return http.Response(
              jsonEncode([
                {
                  'id': product,
                  'puesto_id': puesto,
                  'nombre': 'Tomate',
                  'categoria': 'verduras',
                  'unidad': 'libra',
                  'precio_unitario': 1.25,
                  'activo': true,
                  'revision': '2-rev',
                },
              ]),
              200,
            );
          }
          if (segments.length == 3 &&
              segments[0] == 'puestos' &&
              segments[1] == puesto &&
              segments[2] == 'existencias') {
            return http.Response('[]', 200);
          }
          if (segments.length == 2 &&
              segments[0] == 'productos' &&
              segments[1] == product) {
            return http.Response(
              jsonEncode({
                'id': product,
                'puesto_id': puesto,
                'nombre': 'Tomate',
                'categoria': 'verduras',
                'unidad': 'libra',
                'precio_unitario': 1.25,
                'activo': true,
                'revision': '2-rev',
              }),
              200,
            );
          }
          if (path == '/productos' && request.method == 'POST') {
            return http.Response('{}', 201);
          }
          if (path == '/sync/operaciones') {
            final body = jsonDecode(request.body) as Map<String, dynamic>;
            final op =
                (body['operaciones'] as List).single as Map<String, dynamic>;
            return http.Response(
              jsonEncode({
                'resultados': [
                  {
                    'operacion_id': op['id'],
                    'estado': 'aceptada',
                    'mensaje': 'OK',
                    'incidencia_id': null,
                  },
                ],
              }),
              200,
            );
          }
          if (segments.length == 3 &&
              segments[0] == 'puestos' &&
              segments[1] == puesto &&
              segments[2] == 'usuarios' &&
              expired) {
            return http.Response('{"error":"Sesión vencida"}', 401);
          }
          return http.Response('{}', 200);
        }),
      );

      await controller.login('http://127.0.0.1:3000', user, 'test-password');
      await controller.crearProducto(
        nombre: 'Tomate',
        categoria: 'verduras',
        unidad: 'libra',
        precio: '1.25',
      );
      final previous = await controller.obtenerProducto(product);
      await controller.actualizarProducto(
        previous,
        nombre: 'Tomate rojo',
        categoria: 'verduras',
        unidad: 'libra',
        precio: '1.50',
        activo: true,
      );
      final put = requests.firstWhere((r) => r.method == 'PUT');
      expect(jsonDecode(put.body)['revision'], '2-rev');
      expect(jsonDecode(put.body)['precio_unitario'], 1.5);

      final saved = await controller.registrarOperacion(
        tipo: 'compra',
        renglones: [
          {'producto_id': product, 'cantidad': 2, 'costo_unitario': 0.75},
        ],
      );
      expect(saved.estado, 'aceptada');
      expect((await store.ventas(puesto, user)).single.cuerpo['id'], saved.id);

      expired = true;
      await expectLater(
        controller.listarUsuarios(),
        throwsA(isA<ApiError>().having((e) => e.status, 'status', 401)),
      );
      expect(controller.usuario, isNull);
      expect(await LocalStore.secure.read(key: 'token'), isNull);
      await store.db.close();
    },
  );
}
