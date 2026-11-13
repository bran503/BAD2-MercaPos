import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:sembast_web/sembast_web.dart';

import 'models.dart';

class LocalStore {
  LocalStore._(this.db);

  final Database db;
  static const secure = FlutterSecureStorage();
  static final _products = stringMapStoreFactory.store('productos');
  static final _sales = stringMapStoreFactory.store('ventas');

  static Future<LocalStore> open() async =>
      LocalStore._(await databaseFactoryWeb.openDatabase('mercapos_local'));

  Future<void> guardarProductos(
    String puestoId,
    List<Producto> productos,
  ) async {
    await db.transaction((txn) async {
      final anteriores = await _products.find(
        txn,
        finder: Finder(filter: Filter.equals('puesto_id', puestoId)),
      );
      for (final previo in anteriores) {
        await _products.record(previo.key).delete(txn);
      }
      for (final producto in productos) {
        await _products.record(producto.id).put(txn, {
          'id': producto.id,
          'puesto_id': producto.puestoId,
          'contenido': jsonEncode(producto.toJson()),
        });
      }
    });
  }

  Future<List<Producto>> productos(String puestoId) async {
    final rows = await _products.find(
      db,
      finder: Finder(filter: Filter.equals('puesto_id', puestoId)),
    );
    return rows
        .map(
          (row) => Producto.fromJson(
            jsonDecode(row.value['contenido'] as String)
                as Map<String, dynamic>,
          ),
        )
        .where((producto) => producto.activo)
        .toList()
      ..sort((a, b) => a.nombre.compareTo(b.nombre));
  }

  Future<void> guardarVenta(Map<String, dynamic> cuerpo) async {
    final id = cuerpo['id'] as String;
    await db.transaction((txn) async {
      if (await _sales.record(id).get(txn) != null) {
        throw StateError('La venta ya existe en la cola local');
      }
      await _sales.record(id).put(txn, {
        'id': id,
        'puesto_id': cuerpo['puesto_id'],
        'usuario_id': cuerpo['usuario_id'],
        'fecha': cuerpo['fecha_local'],
        'cuerpo': jsonEncode(cuerpo),
        'estado': 'pendiente',
      });
    });
  }

  Filter _owner(String puestoId, String usuarioId) => Filter.and([
    Filter.equals('puesto_id', puestoId),
    Filter.equals('usuario_id', usuarioId),
  ]);

  Future<List<VentaLocal>> ventas(String puestoId, String usuarioId) async {
    final rows = await _sales.find(
      db,
      finder: Finder(
        filter: _owner(puestoId, usuarioId),
        sortOrders: [SortOrder('fecha', false)],
      ),
    );
    return rows.map((row) => VentaLocal.fromRow(row.value)).toList();
  }

  Future<List<VentaLocal>> pendientes(String puestoId, String usuarioId) async {
    final rows = await _sales.find(
      db,
      finder: Finder(
        filter: Filter.and([
          _owner(puestoId, usuarioId),
          Filter.equals('estado', 'pendiente'),
        ]),
        sortOrders: [SortOrder('fecha')],
        limit: 100,
      ),
    );
    return rows.map((row) => VentaLocal.fromRow(row.value)).toList();
  }

  Future<void> aplicarResultados(
    String puestoId,
    String usuarioId,
    List<VentaLocal> enviadas,
    List<dynamic> resultados,
  ) async {
    final ids = enviadas.map((venta) => venta.id).toSet();
    if (resultados.length != ids.length) {
      throw const FormatException('Respuesta de sincronización incompleta');
    }
    final vistos = <String>{};
    for (final item in resultados) {
      if (item is! Map<String, dynamic>) {
        throw const FormatException('Resultado inválido');
      }
      final id = item['operacion_id'];
      final estado = item['estado'];
      if (id is! String ||
          !ids.contains(id) ||
          !vistos.add(id) ||
          ![
            'aceptada',
            'aceptada_con_incidencia',
            'rechazada',
          ].contains(estado)) {
        throw const FormatException(
          'Resultado de sincronización inconsistente',
        );
      }
    }
    await db.transaction((txn) async {
      for (final item in resultados.cast<Map<String, dynamic>>()) {
        final id = item['operacion_id'] as String;
        final previo = await _sales.record(id).get(txn);
        if (previo == null ||
            previo['puesto_id'] != puestoId ||
            previo['usuario_id'] != usuarioId ||
            previo['estado'] != 'pendiente') {
          throw StateError('La cola cambió durante la sincronización');
        }
        await _sales.record(id).put(txn, {
          ...previo,
          'estado': item['estado'],
          'mensaje': item['mensaje']?.toString(),
          'incidencia_id': item['incidencia_id']?.toString(),
        });
      }
    });
  }

  Future<void> reintentar(String id, String puestoId, String usuarioId) async {
    await db.transaction((txn) async {
      final previo = await _sales.record(id).get(txn);
      if (previo == null ||
          previo['puesto_id'] != puestoId ||
          previo['usuario_id'] != usuarioId ||
          previo['estado'] != 'rechazada') {
        return;
      }
      await _sales.record(id).put(txn, {...previo, 'estado': 'pendiente'});
    });
  }
}
