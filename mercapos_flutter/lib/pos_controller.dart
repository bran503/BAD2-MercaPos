import 'dart:convert';
import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

import 'local_store.dart';
import 'models.dart';

class ApiError implements Exception {
  const ApiError(this.message, [this.status]);
  final String message;
  final int? status;

  @override
  String toString() => message;
}

class PosController extends ChangeNotifier {
  PosController(this.store, {http.Client? client})
    : _client = client ?? http.Client();

  final LocalStore store;
  final http.Client _client;
  Usuario? usuario;
  String? token;
  String baseUrl = kIsWeb ? 'http://127.0.0.1:3000' : 'http://10.0.2.2:3000';
  List<Producto> productos = [];
  List<VentaLocal> ventas = [];
  Map<String, int> existenciasMil = {};
  String? aviso;
  bool sincronizando = false;

  Future<void> iniciar() async {
    baseUrl = await LocalStore.secure.read(key: 'api_url') ?? baseUrl;
    token = await LocalStore.secure.read(key: 'token');
    final data = await LocalStore.secure.read(key: 'usuario');
    if (data != null) {
      usuario = Usuario.fromJson(jsonDecode(data) as Map<String, dynamic>);
    }
    await cargarLocal();
    if (token != null && usuario != null) await conectar();
  }

  Future<void> cargarLocal() async {
    final actual = usuario;
    if (actual == null) return;
    productos = await store.productos(actual.puestoId);
    ventas = await store.ventas(actual.puestoId, actual.id);
    notifyListeners();
  }

  Future<void> login(String url, String usuarioId, String password) async {
    final parsed = Uri.tryParse(url.trim());
    if (parsed == null ||
        !parsed.hasAuthority ||
        !['http', 'https'].contains(parsed.scheme) ||
        parsed.path != '' ||
        parsed.hasQuery ||
        parsed.hasFragment) {
      throw const ApiError('Escribe una URL como http://IP:3000');
    }
    baseUrl = url.trim().replaceFirst(RegExp(r'/$'), '');
    final nombreUsuario = usuarioId.trim();
    final idUsuario = nombreUsuario.startsWith('usuario:')
        ? nombreUsuario
        : 'usuario:$nombreUsuario';
    final body = await _request('POST', '/auth/login', {
      'usuario_id': idUsuario,
      'password': password,
    }, authorized: false);
    token = body['token'] as String;
    usuario = Usuario.fromJson(body['usuario'] as Map<String, dynamic>);
    await LocalStore.secure.write(key: 'api_url', value: baseUrl);
    await LocalStore.secure.write(key: 'token', value: token);
    await LocalStore.secure.write(
      key: 'usuario',
      value: jsonEncode(usuario!.toJson()),
    );
    await cargarLocal();
    await conectar();
  }

  Future<void> conectar() async {
    try {
      await actualizarCatalogo();
    } catch (e) {
      aviso = 'Catálogo sin actualizar: ${_mensaje(e)}';
      notifyListeners();
    }
    try {
      await sincronizar();
    } catch (_) {
      // Sincronizar deja el motivo visible y conserva la cola.
    }
  }

  Future<void> logout() async {
    try {
      await _request('POST', '/auth/logout', null);
    } catch (_) {
      // El token local se descarta incluso si el servidor no está disponible.
    }
    await LocalStore.secure.delete(key: 'token');
    await LocalStore.secure.delete(key: 'usuario');
    token = null;
    usuario = null;
    productos = [];
    ventas = [];
    existenciasMil = {};
    aviso = null;
    notifyListeners();
  }

  Future<void> actualizarCatalogo() async {
    final actual = usuario;
    if (actual == null) return;
    final query = Uri(
      queryParameters: {'puesto_id': actual.puestoId, 'activo': 'true'},
    ).query;
    final body = await _request('GET', '/productos?$query', null);
    if (body is! List) throw const ApiError('Formato de catálogo inesperado');
    final nuevos = body
        .map((item) => Producto.fromJson(item as Map<String, dynamic>))
        .where((p) => p.puestoId == actual.puestoId)
        .toList();
    await store.guardarProductos(actual.puestoId, nuevos);
    productos = await store.productos(actual.puestoId);
    await actualizarExistencias();
    aviso = null;
    notifyListeners();
  }

  Future<void> actualizarExistencias() async {
    final actual = usuario;
    if (actual == null) return;
    final body = await _request(
      'GET',
      '/puestos/${Uri.encodeComponent(actual.puestoId)}/existencias',
      null,
    );
    if (body is! List) {
      throw const ApiError('Formato de existencias inesperado');
    }
    existenciasMil = {
      for (final item in body.cast<Map<String, dynamic>>())
        item['producto_id'] as String: (item['existencia_mil'] as num).toInt(),
    };
    notifyListeners();
  }

  Future<String> guardarVenta(
    Map<String, int> carrito, {
    Map<String, dynamic>? pago,
  }) async {
    final actual = usuario;
    if (actual == null) throw const ApiError('Inicia sesión primero');
    if (carrito.isEmpty) throw const ApiError('Agrega productos a la venta');
    if (carrito.length > 100) {
      throw const ApiError('Máximo 100 productos por venta');
    }
    final porId = {for (final producto in productos) producto.id: producto};
    final renglones = <Map<String, dynamic>>[];
    for (final entry in carrito.entries) {
      final producto = porId[entry.key];
      if (producto == null) {
        throw const ApiError('Producto no disponible en catálogo local');
      }
      if (entry.value <= 0 || entry.value > 1000000000) {
        throw const ApiError('Cantidad fuera del límite permitido');
      }
      renglones.add({
        'producto_id': producto.id,
        'cantidad': num.parse(scaledText(entry.value, 3)),
        'precio_unitario': producto.precio,
      });
    }
    final cuerpo = <String, dynamic>{
      'id': 'operacion:${_uuid()}',
      'tipo_operacion': 'venta',
      'puesto_id': actual.puestoId,
      'usuario_id': actual.id,
      'dispositivo_id': await _dispositivoId(),
      'fecha_local': _fechaConZona(DateTime.now()),
      'renglones': renglones,
    };
    if (pago != null) cuerpo['pago'] = pago;
    // La venta se confirma en almacenamiento local antes de cualquier intento de red.
    await store.guardarVenta(cuerpo);
    await cargarLocal();
    return cuerpo['id'] as String;
  }

  Future<void> sincronizar() async {
    final actual = usuario;
    if (actual == null || token == null || sincronizando) return;
    sincronizando = true;
    notifyListeners();
    try {
      var procesadas = false;
      while (true) {
        final lote = await store.pendientes(actual.puestoId, actual.id);
        if (lote.isEmpty) break;
        final body = await _request('POST', '/sync/operaciones', {
          'operaciones': _ordenarDependencias(
            lote,
          ).map((operacion) => operacion.cuerpo).toList(),
        });
        if (body is! Map<String, dynamic> || body['resultados'] is! List) {
          throw const ApiError('Respuesta de sincronización inválida');
        }
        await store.aplicarResultados(
          actual.puestoId,
          actual.id,
          lote,
          body['resultados'] as List<dynamic>,
        );
        procesadas = true;
        await cargarLocal();
        if (lote.length < 100) break;
      }
      if (procesadas) {
        try {
          await actualizarExistencias();
          aviso = null;
        } catch (e) {
          aviso = 'Operaciones sincronizadas; saldo sin actualizar: ${_mensaje(e)}';
        }
      }
    } catch (e) {
      aviso = 'Sincronización pendiente: ${_mensaje(e)}';
      rethrow;
    } finally {
      sincronizando = false;
      await cargarLocal();
      notifyListeners();
    }
  }

  Future<void> reintentar(VentaLocal venta) async {
    final actual = usuario;
    if (actual == null) return;
    await store.reintentar(venta.id, actual.puestoId, actual.id);
    await cargarLocal();
    await sincronizar();
  }

  List<VentaLocal> _ordenarDependencias(List<VentaLocal> lote) {
    final porId = {for (final op in lote) op.id: op};
    final visitados = <String>{};
    final activos = <String>{};
    final ordenados = <VentaLocal>[];
    void visitar(VentaLocal op) {
      if (visitados.contains(op.id) || !activos.add(op.id)) return;
      final origen = porId[op.cuerpo['operacion_origen_id']];
      if (origen != null) visitar(origen);
      activos.remove(op.id);
      visitados.add(op.id);
      ordenados.add(op);
    }

    for (final op in lote) {
      visitar(op);
    }
    return ordenados;
  }

  bool get esAdministrador => usuario?.rol == 'administrador';

  Future<List<Map<String, dynamic>>> listarProductos() async {
    final actual = usuario!;
    final query = Uri(queryParameters: {'puesto_id': actual.puestoId}).query;
    return _lista(await _request('GET', '/productos?$query', null));
  }

  Future<Map<String, dynamic>> obtenerProducto(String id) async => _mapa(
    await _request('GET', '/productos/${Uri.encodeComponent(id)}', null),
  );

  Future<void> crearProducto({
    required String nombre,
    required String categoria,
    required String unidad,
    required String precio,
  }) async {
    final actual = usuario!;
    final valor = parseScaled(precio, 4);
    if (valor < 0 || valor > 10000000000) {
      throw const ApiError('Precio fuera del límite permitido');
    }
    await _request('POST', '/productos', {
      'id': 'producto:${_uuid()}',
      'puesto_id': actual.puestoId,
      'nombre': nombre.trim(),
      'categoria': categoria.trim(),
      'precio_unitario': num.parse(scaledText(valor, 4)),
      'unidad': unidad.trim(),
      'actualizado_en': _fechaConZona(DateTime.now()),
      'actualizado_por': actual.id,
    });
    await actualizarCatalogo();
  }

  Future<void> actualizarProducto(
    Map<String, dynamic> previo, {
    required String nombre,
    required String categoria,
    required String unidad,
    required String precio,
    required bool activo,
  }) async {
    final revision = previo['revision'];
    if (revision is! String) {
      throw const ApiError('Falta revisión del producto');
    }
    final valor = parseScaled(precio, 4);
    if (valor < 0 || valor > 10000000000) {
      throw const ApiError('Precio fuera del límite permitido');
    }
    await _request(
      'PUT',
      '/productos/${Uri.encodeComponent(previo['id'] as String)}',
      {
        'revision': revision,
        'puesto_id': usuario!.puestoId,
        'nombre': nombre.trim(),
        'categoria': categoria.trim(),
        'precio_unitario': num.parse(scaledText(valor, 4)),
        'unidad': unidad.trim(),
        'activo': activo,
        'actualizado_en': _fechaConZona(DateTime.now()),
        'actualizado_por': usuario!.id,
      },
    );
    await actualizarCatalogo();
  }

  Future<void> abrirInventario(String productoId, int cantidadMil) async {
    if (!esAdministrador) {
      throw const ApiError('Solo el administrador puede abrir inventario');
    }
    if (cantidadMil <= 0) throw const ApiError('La cantidad debe ser positiva');
    final actual = usuario!;
    await _request('POST', '/movimientos', {
      'id': 'mov:inicio:$productoId',
      'producto_id': productoId,
      'puesto_id': actual.puestoId,
      'cantidad': num.parse(scaledText(cantidadMil, 3)),
      'motivo': 'inventario_inicial',
      'dispositivo_id': await _dispositivoId(),
      'usuario_id': actual.id,
      'fecha_local': _fechaConZona(DateTime.now()),
    });
    await actualizarExistencias();
  }

  Future<List<Map<String, dynamic>>> listarMovimientos() async {
    final query = Uri(queryParameters: {'puesto_id': usuario!.puestoId}).query;
    return _lista(await _request('GET', '/movimientos?$query', null));
  }

  Future<List<Map<String, dynamic>>> listarOperaciones() async {
    final query = Uri(queryParameters: {'puesto_id': usuario!.puestoId}).query;
    return _lista(await _request('GET', '/operaciones?$query', null));
  }

  Future<VentaLocal> registrarOperacion({
    required String tipo,
    required List<Map<String, dynamic>> renglones,
    String? motivo,
    String? origenId,
    String? tipoDevolucion,
    String? destinoDevolucion,
    String? conteoId,
  }) async {
    final actual = usuario!;
    if (renglones.isEmpty || renglones.length > 100) {
      throw const ApiError('Agrega entre 1 y 100 renglones');
    }
    final id = 'operacion:${_uuid()}';
    final cuerpo = <String, dynamic>{
      'id': id,
      'tipo_operacion': tipo,
      'puesto_id': actual.puestoId,
      'usuario_id': actual.id,
      'dispositivo_id': await _dispositivoId(),
      'fecha_local': _fechaConZona(DateTime.now()),
      'renglones': renglones,
      if (motivo != null) 'motivo': motivo.trim(),
      'operacion_origen_id': ?origenId,
      'tipo_devolucion': ?tipoDevolucion,
      'destino_devolucion': ?destinoDevolucion,
      'conteo_id': ?conteoId,
    };
    await store.guardarVenta(cuerpo);
    await cargarLocal();
    try {
      await sincronizar();
    } catch (_) {
      // El registro local permanece pendiente y se puede reintentar.
    }
    return ventas.firstWhere((op) => op.id == id);
  }

  Future<List<Map<String, dynamic>>> listarConteos() async {
    final query = Uri(queryParameters: {'puesto_id': usuario!.puestoId}).query;
    return _lista(await _request('GET', '/conteos?$query', null));
  }

  Future<Map<String, dynamic>> crearConteo({
    required String periodo,
    required String etapa,
    required String origen,
    required List<Map<String, dynamic>> renglones,
    String? evidencia,
  }) async {
    if (periodo.trim().isEmpty || renglones.isEmpty) {
      throw const ApiError('Indica período y al menos un producto');
    }
    final actual = usuario!;
    return _mapa(
      await _request('POST', '/conteos', {
        'id': 'conteo:${_uuid()}',
        'puesto_id': actual.puestoId,
        'usuario_id': actual.id,
        'dispositivo_id': await _dispositivoId(),
        'fecha_local': _fechaConZona(DateTime.now()),
        'periodo_estudio': periodo.trim(),
        'etapa': etapa,
        'origen_registro': origen,
        if (evidencia != null) 'evidencia': evidencia.trim(),
        'renglones': renglones,
      }),
    );
  }

  Future<VentaLocal> ajustarConteo(Map<String, dynamic> conteo) async {
    if (!esAdministrador) {
      throw const ApiError('Solo el administrador puede ajustar');
    }
    final id = conteo['id'] as String;
    final operaciones = await listarOperaciones();
    if (operaciones.any((op) =>
        op['tipo_operacion'] == 'ajuste' && op['conteo_id'] == id)) {
      throw const ApiError('Este conteo ya tiene un ajuste');
    }
    if (ventas.any((op) => op.cuerpo['conteo_id'] == id && op.estado != 'rechazada')) {
      throw const ApiError('Este conteo ya tiene un ajuste pendiente o aceptado');
    }
    final renglones = <Map<String, dynamic>>[];
    for (final row in _lista(conteo['renglones'])) {
      final diferencia = row['diferencia'] as num;
      if (diferencia != 0) {
        renglones.add({
          'producto_id': row['producto_id'],
          'cantidad': diferencia,
        });
      }
    }
    if (renglones.isEmpty) {
      throw const ApiError('El conteo no tiene diferencias');
    }
    return registrarOperacion(
      tipo: 'ajuste',
      renglones: renglones,
      motivo: 'Diferencia detectada en conteo físico',
      conteoId: id,
    );
  }

  Future<List<Map<String, dynamic>>> listarUsuarios() async => _lista(
    await _request(
      'GET',
      '/puestos/${Uri.encodeComponent(usuario!.puestoId)}/usuarios',
      null,
    ),
  );

  Future<void> crearUsuario({
    required String id,
    required String nombre,
    required String rol,
    required String password,
  }) async {
    final limpio = id.trim();
    await _request('POST', '/usuarios', {
      'id': limpio.startsWith('usuario:') ? limpio : 'usuario:$limpio',
      'nombre': nombre.trim(),
      'rol': rol,
      'puesto_id': usuario!.puestoId,
      'activo': true,
      'password': password,
    });
  }

  Future<void> actualizarUsuario(
    Map<String, dynamic> previo, {
    required String nombre,
    required String rol,
    required bool activo,
    String? password,
  }) async {
    await _request(
      'PUT',
      '/usuarios/${Uri.encodeComponent(previo['id'] as String)}',
      {
        'id': previo['id'],
        'revision': previo['revision'],
        'nombre': nombre.trim(),
        'rol': rol,
        'puesto_id': usuario!.puestoId,
        'activo': activo,
        if (password != null && password.isNotEmpty) 'password': password,
      },
    );
  }

  Future<Map<String, dynamic>> obtenerPuesto() async => _mapa(
    await _request(
      'GET',
      '/puestos/${Uri.encodeComponent(usuario!.puestoId)}',
      null,
    ),
  );

  Future<void> actualizarPuesto(
    Map<String, dynamic> previo, {
    required String nombre,
    required String mercado,
    required String ubicacion,
    required bool activo,
  }) async {
    await _request(
      'PUT',
      '/puestos/${Uri.encodeComponent(usuario!.puestoId)}',
      {
        'id': usuario!.puestoId,
        'revision': previo['revision'],
        'nombre': nombre.trim(),
        'mercado': mercado.trim(),
        'ubicacion': ubicacion.trim(),
        'propietario_usuario_id': previo['propietario_usuario_id'],
        'activo': activo,
      },
    );
  }

  Future<void> cambiarPassword(String actual, String nueva) async {
    await _request('POST', '/auth/password', {
      'actual': actual,
      'nueva': nueva,
    });
  }

  Future<List<Map<String, dynamic>>> listarIncidencias() async {
    final query = Uri(queryParameters: {'puesto_id': usuario!.puestoId}).query;
    final body = _mapa(await _request('GET', '/sync/incidencias?$query', null));
    return _lista(body['incidencias']);
  }

  Future<void> resolverIncidencia(String operacionId, String nota) async {
    await _request(
      'PUT',
      '/sync/incidencias/${Uri.encodeComponent(operacionId)}/resolver',
      {'nota': nota.trim()},
    );
  }

  List<Map<String, dynamic>> _lista(dynamic value) {
    if (value is! List) throw const ApiError('Lista inesperada de la API');
    return value.map((item) => _mapa(item)).toList();
  }

  Map<String, dynamic> _mapa(dynamic value) {
    if (value is! Map<String, dynamic>) {
      throw const ApiError('Respuesta inesperada de la API');
    }
    return value;
  }

  Future<dynamic> _request(
    String method,
    String endpoint,
    Object? body, {
    bool authorized = true,
  }) async {
    final headers = <String, String>{'Accept': 'application/json'};
    if (body != null) headers['Content-Type'] = 'application/json';
    if (authorized) {
      if (token == null) throw const ApiError('Inicia sesión para sincronizar');
      headers['Authorization'] = 'Bearer $token';
    }
    final uri = Uri.parse('$baseUrl$endpoint');
    late http.Response response;
    try {
      final Future<http.Response> request;
      if (method == 'GET') {
        request = _client.get(uri, headers: headers);
      } else if (method == 'POST') {
        request = _client.post(
          uri,
          headers: headers,
          body: body == null ? null : jsonEncode(body),
        );
      } else if (method == 'PUT') {
        request = _client.put(
          uri,
          headers: headers,
          body: body == null ? null : jsonEncode(body),
        );
      } else {
        throw const ApiError('Método HTTP no admitido');
      }
      response = await request.timeout(const Duration(seconds: 12));
    } catch (e) {
      throw ApiError('No se pudo contactar la API: $e');
    }
    dynamic data;
    try {
      data = response.body.isEmpty ? null : jsonDecode(response.body);
    } catch (_) {
      throw ApiError('Respuesta no JSON de la API', response.statusCode);
    }
    if (response.statusCode >= 400) {
      final message = data is Map ? data['error']?.toString() : null;
      if (response.statusCode == 401) {
        if (endpoint == '/auth/login') {
          throw ApiError(message ?? 'Usuario o contraseña incorrectos', 401);
        }
        await LocalStore.secure.delete(key: 'token');
        await LocalStore.secure.delete(key: 'usuario');
        token = null;
        usuario = null;
        productos = [];
        ventas = [];
        existenciasMil = {};
        notifyListeners();
        throw const ApiError('La sesión venció. Inicia sesión de nuevo.', 401);
      }
      throw ApiError(
        message ?? 'Error HTTP ${response.statusCode}',
        response.statusCode,
      );
    }
    return data;
  }

  String _mensaje(Object error) =>
      error is ApiError ? error.message : error.toString();

  Future<String> _dispositivoId() async {
    final existing = await LocalStore.secure.read(key: 'dispositivo_id');
    if (existing != null) return existing;
    final value = 'pos-${_uuid()}';
    await LocalStore.secure.write(key: 'dispositivo_id', value: value);
    return value;
  }

  String _uuid() {
    final random = Random.secure();
    final bytes = List<int>.generate(16, (_) => random.nextInt(256));
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    final hex = bytes
        .map((byte) => byte.toRadixString(16).padLeft(2, '0'))
        .join();
    return '${hex.substring(0, 8)}-${hex.substring(8, 12)}-${hex.substring(12, 16)}-'
        '${hex.substring(16, 20)}-${hex.substring(20)}';
  }

  String _fechaConZona(DateTime date) {
    final offset = date.timeZoneOffset;
    final minutes = offset.inMinutes.abs();
    final sign = offset.isNegative ? '-' : '+';
    final hh = (minutes ~/ 60).toString().padLeft(2, '0');
    final mm = (minutes % 60).toString().padLeft(2, '0');
    return '${date.toIso8601String()}$sign$hh:$mm';
  }
}
