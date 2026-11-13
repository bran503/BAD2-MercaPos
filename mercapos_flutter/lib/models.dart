import 'dart:convert';

class Usuario {
  const Usuario({
    required this.id,
    required this.nombre,
    required this.rol,
    required this.puestoId,
  });

  final String id;
  final String nombre;
  final String rol;
  final String puestoId;

  factory Usuario.fromJson(Map<String, dynamic> json) => Usuario(
    id: json['id'] as String,
    nombre: json['nombre'] as String,
    rol: json['rol'] as String,
    puestoId: json['puesto_id'] as String,
  );

  Map<String, dynamic> toJson() => {
    'id': id,
    'nombre': nombre,
    'rol': rol,
    'puesto_id': puestoId,
  };
}

class Producto {
  const Producto({
    required this.id,
    required this.puestoId,
    required this.nombre,
    required this.categoria,
    required this.unidad,
    required this.precio,
    required this.activo,
  });

  final String id;
  final String puestoId;
  final String nombre;
  final String categoria;
  final String unidad;
  final num precio;
  final bool activo;

  factory Producto.fromJson(Map<String, dynamic> json) => Producto(
    id: json['id'] as String,
    puestoId: json['puesto_id'] as String,
    nombre: json['nombre'] as String,
    categoria: json['categoria'] as String,
    unidad: json['unidad'] as String,
    precio: json['precio_unitario'] as num,
    activo: json['activo'] as bool? ?? true,
  );

  Map<String, dynamic> toJson() => {
    'id': id,
    'puesto_id': puestoId,
    'nombre': nombre,
    'categoria': categoria,
    'unidad': unidad,
    'precio_unitario': precio,
    'activo': activo,
  };
}

class VentaLocal {
  const VentaLocal({
    required this.id,
    required this.puestoId,
    required this.usuarioId,
    required this.fecha,
    required this.cuerpo,
    required this.estado,
    this.mensaje,
    this.incidenciaId,
  });

  final String id;
  final String puestoId;
  final String usuarioId;
  final String fecha;
  final Map<String, dynamic> cuerpo;
  final String estado;
  final String? mensaje;
  final String? incidenciaId;

  factory VentaLocal.fromRow(Map<String, Object?> row) => VentaLocal(
    id: row['id'] as String,
    puestoId: row['puesto_id'] as String,
    usuarioId: row['usuario_id'] as String,
    fecha: row['fecha'] as String,
    cuerpo: jsonDecode(row['cuerpo'] as String) as Map<String, dynamic>,
    estado: row['estado'] as String,
    mensaje: row['mensaje'] as String?,
    incidenciaId: row['incidencia_id'] as String?,
  );
}

int parseScaled(String text, int digits) {
  final clean = text.trim().replaceAll(',', '.');
  final match = RegExp(r'^(\d+)(?:\.(\d+))?$').firstMatch(clean);
  if (match == null) {
    throw const FormatException('Escribe una cantidad positiva');
  }
  final fraction = match.group(2) ?? '';
  if (fraction.length > digits) {
    throw FormatException('Máximo $digits decimales');
  }
  final scale = _power10(digits);
  return int.parse(match.group(1)!) * scale +
      int.parse(
        fraction.padRight(digits, '0').isEmpty
            ? '0'
            : fraction.padRight(digits, '0'),
      );
}

int _power10(int digits) {
  var result = 1;
  for (var i = 0; i < digits; i++) {
    result *= 10;
  }
  return result;
}

String scaledText(int value, int digits) {
  final scale = _power10(digits);
  final absolute = value.abs();
  final sign = value < 0 ? '-' : '';
  final whole = absolute ~/ scale;
  final fraction = (absolute % scale)
      .toString()
      .padLeft(digits, '0')
      .replaceFirst(RegExp(r'0+$'), '');
  return fraction.isEmpty ? '$sign$whole' : '$sign$whole.$fraction';
}

int precioDiezmiles(num value) => parseScaled(value.toString(), 4);

// Cantidad en milésimas × precio en diezmilésimas, redondeado al centavo.
int subtotalCentavos(int cantidadMil, int precioDiezMil) =>
    ((BigInt.from(cantidadMil) * BigInt.from(precioDiezMil) +
                BigInt.from(50000)) ~/
            BigInt.from(100000))
        .toInt();

String dinero(int centavos) =>
    '\$${centavos ~/ 100}.${(centavos % 100).toString().padLeft(2, '0')}';
