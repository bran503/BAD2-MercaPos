import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:path/path.dart' as path;
import 'package:sqflite/sqflite.dart';

import 'models.dart';

class LocalStore {
  LocalStore._(this.db);

  final Database db;
  static const secure = FlutterSecureStorage();

  static Future<LocalStore> open() async {
    final file = path.join(await getDatabasesPath(), 'mercapos_local.db');
    final db = await openDatabase(
      file,
      version: 1,
      onCreate: (db, _) async {
        await db.execute('''CREATE TABLE productos (
          id TEXT PRIMARY KEY, puesto_id TEXT NOT NULL, contenido TEXT NOT NULL
        )''');
        await db.execute(
          'CREATE INDEX productos_puesto ON productos(puesto_id)',
        );
        await db.execute('''CREATE TABLE ventas (
          id TEXT PRIMARY KEY, puesto_id TEXT NOT NULL, usuario_id TEXT NOT NULL,
          fecha TEXT NOT NULL, cuerpo TEXT NOT NULL, estado TEXT NOT NULL,
          mensaje TEXT, incidencia_id TEXT
        )''');
        await db.execute(
          'CREATE INDEX ventas_cola ON ventas(puesto_id, usuario_id, estado, fecha)',
        );
      },
    );
    return LocalStore._(db);
  }

  Future<void> guardarProductos(
    String puestoId,
    List<Producto> productos,
  ) async {
    await db.transaction((txn) async {
      await txn.delete(
        'productos',
        where: 'puesto_id = ?',
        whereArgs: [puestoId],
      );
      for (final producto in productos) {
        await txn.insert('productos', {
          'id': producto.id,
          'puesto_id': producto.puestoId,
          'contenido': jsonEncode(producto.toJson()),
        });
      }
    });
  }

  Future<List<Producto>> productos(String puestoId) async {
    final rows = await db.query(
      'productos',
      where: 'puesto_id = ?',
      whereArgs: [puestoId],
    );
    return rows
        .map(
          (row) => Producto.fromJson(
            jsonDecode(row['contenido'] as String) as Map<String, dynamic>,
          ),
        )
        .where((producto) => producto.activo)
        .toList()
      ..sort((a, b) => a.nombre.compareTo(b.nombre));
  }

  Future<void> guardarVenta(Map<String, dynamic> cuerpo) async {
    await db.insert('ventas', {
      'id': cuerpo['id'],
      'puesto_id': cuerpo['puesto_id'],
      'usuario_id': cuerpo['usuario_id'],
      'fecha': cuerpo['fecha_local'],
      'cuerpo': jsonEncode(cuerpo),
      'estado': 'pendiente',
    });
  }

  Future<List<VentaLocal>> ventas(String puestoId, String usuarioId) async {
    final rows = await db.query(
      'ventas',
      where: 'puesto_id = ? AND usuario_id = ?',
      whereArgs: [puestoId, usuarioId],
      orderBy: 'fecha DESC',
    );
    return rows.map(VentaLocal.fromRow).toList();
  }

  Future<List<VentaLocal>> pendientes(String puestoId, String usuarioId) async {
    final rows = await db.query(
      'ventas',
      where: 'puesto_id = ? AND usuario_id = ? AND estado = ?',
      whereArgs: [puestoId, usuarioId, 'pendiente'],
      orderBy: 'fecha ASC',
      limit: 100,
    );
    return rows.map(VentaLocal.fromRow).toList();
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
        final changed = await txn.update(
          'ventas',
          {
            'estado': item['estado'],
            'mensaje': item['mensaje']?.toString(),
            'incidencia_id': item['incidencia_id']?.toString(),
          },
          where: 'id = ? AND puesto_id = ? AND usuario_id = ? AND estado = ?',
          whereArgs: [item['operacion_id'], puestoId, usuarioId, 'pendiente'],
        );
        if (changed != 1) {
          throw StateError('La cola cambió durante la sincronización');
        }
      }
    });
  }

  Future<void> reintentar(String id, String puestoId, String usuarioId) async {
    await db.update(
      'ventas',
      {'estado': 'pendiente'},
      where: 'id = ? AND puesto_id = ? AND usuario_id = ? AND estado = ?',
      whereArgs: [id, puestoId, usuarioId, 'rechazada'],
    );
  }
}
