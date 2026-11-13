import 'dart:convert';
import 'dart:math';

import 'package:flutter/material.dart';

import 'impresion.dart';
import 'models.dart';

class ResultadoPago {
  const ResultadoPago({
    required this.estado,
    required this.autorizacion,
    required this.fecha,
  });

  final String estado;
  final String autorizacion;
  final DateTime fecha;

  bool get aprobado => estado == 'aprobado';

  Map<String, dynamic> toJson() => {
    'metodo': 'tarjeta',
    'modo': 'simulado',
    'estado': estado,
    'autorizacion': autorizacion,
    'fecha': fechaConZona(fecha),
  };
}

class LineaComprobante {
  const LineaComprobante({
    required this.nombre,
    required this.cantidad,
    required this.unidad,
    required this.importe,
  });

  final String nombre;
  final String cantidad;
  final String unidad;
  final String importe;
}

String fechaConZona(DateTime date) {
  final offset = date.timeZoneOffset;
  final minutes = offset.inMinutes.abs();
  final sign = offset.isNegative ? '-' : '+';
  final hh = (minutes ~/ 60).toString().padLeft(2, '0');
  final mm = (minutes % 60).toString().padLeft(2, '0');
  return '${date.toIso8601String()}$sign$hh:$mm';
}

String _fechaCorta(DateTime date) {
  String two(int value) => value.toString().padLeft(2, '0');
  return '${two(date.day)}/${two(date.month)}/${date.year} '
      '${two(date.hour)}:${two(date.minute)}';
}

String folioCorto(String operacionId) {
  final limpio = operacionId.replaceFirst('operacion:', '').toUpperCase();
  return limpio.length <= 10 ? limpio : limpio.substring(limpio.length - 10);
}

Future<ResultadoPago?> mostrarPagoTarjeta(
  BuildContext context, {
  required int total,
}) {
  return showDialog<ResultadoPago>(
    context: context,
    barrierDismissible: false,
    builder: (_) => _PosFicticio(total: total),
  );
}

class _PosFicticio extends StatefulWidget {
  const _PosFicticio({required this.total});
  final int total;

  @override
  State<_PosFicticio> createState() => _PosFicticioState();
}

class _PosFicticioState extends State<_PosFicticio> {
  String seleccion = 'aprobado';
  bool procesando = false;
  ResultadoPago? resultado;

  Future<void> simular() async {
    if (procesando || resultado != null) return;
    setState(() => procesando = true);
    await Future<void>.delayed(const Duration(milliseconds: 900));
    if (!mounted) return;
    final random = Random();
    setState(() {
      procesando = false;
      resultado = ResultadoPago(
        estado: seleccion,
        autorizacion: 'AUT-${100000 + random.nextInt(900000)}',
        fecha: DateTime.now(),
      );
    });
  }

  void reintentar() => setState(() {
    resultado = null;
    seleccion = 'aprobado';
  });

  @override
  Widget build(BuildContext context) {
    final resultado = this.resultado;
    return AlertDialog(
      title: const Text('Pago con tarjeta'),
      content: SizedBox(
        width: 400,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _Tarjeta(total: widget.total),
              const SizedBox(height: 16),
              if (resultado == null)
                ..._espera(context)
              else
                ..._resultado(context, resultado),
            ],
          ),
        ),
      ),
      actions: resultado == null
          ? [
              TextButton(
                onPressed: procesando ? null : () => Navigator.pop(context),
                child: const Text('Cancelar'),
              ),
              FilledButton(
                onPressed: procesando ? null : simular,
                child: procesando
                    ? const SizedBox(
                        height: 18,
                        width: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Text('Proceder pago'),
              ),
            ]
          : [
              if (resultado.estado != 'aprobado')
                TextButton(
                  onPressed: reintentar,
                  child: const Text('Reintentar'),
                ),
              FilledButton(
                onPressed: () => Navigator.pop(
                  context,
                  resultado.aprobado ? resultado : null,
                ),
                child: Text(
                  resultado.aprobado ? 'Usar pago aprobado' : 'Cerrar',
                ),
              ),
            ],
    );
  }

  List<Widget> _espera(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return [
      Text(
        'Acerca, inserta o desliza la tarjeta',
        textAlign: TextAlign.center,
        style: theme.textTheme.titleMedium?.copyWith(
          fontWeight: FontWeight.w600,
        ),
      ),
      const SizedBox(height: 4),
      Text(
        'Pago en proceso',
        textAlign: TextAlign.center,
        style: theme.textTheme.bodySmall?.copyWith(
          color: scheme.onSurfaceVariant,
        ),
      ),
      const SizedBox(height: 18),
      Text(
        'Resultado de la operacion',
        style: theme.textTheme.labelLarge?.copyWith(
          color: scheme.onSurfaceVariant,
        ),
      ),
      const SizedBox(height: 8),
      Wrap(
        alignment: WrapAlignment.center,
        spacing: 8,
        runSpacing: 8,
        children: [
          for (final opcion in const [
            ('aprobado', 'Aprobado', Icons.check_circle_outline),
            ('rechazado', 'Rechazado', Icons.cancel_outlined),
            ('cancelado', 'Cancelado', Icons.block),
          ])
            ChoiceChip(
              avatar: Icon(opcion.$3, size: 18),
              label: Text(opcion.$2),
              selected: seleccion == opcion.$1,
              onSelected: procesando
                  ? null
                  : (_) => setState(() => seleccion = opcion.$1),
            ),
        ],
      ),
      if (procesando) ...[
        const SizedBox(height: 18),
        const Center(child: CircularProgressIndicator()),
        const SizedBox(height: 8),
        const Text('Procesando pago…', textAlign: TextAlign.center),
      ],
    ];
  }

  List<Widget> _resultado(BuildContext context, ResultadoPago resultado) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final (IconData icon, Color color, String titulo, String detalle) =
        switch (resultado.estado) {
          'aprobado' => (
            Icons.check_circle,
            const Color(0xff1b7f4b),
            'Pago aprobado',
            'Al confirmar se guardará la venta y se generará el comprobante.',
          ),
          'rechazado' => (
            Icons.cancel,
            scheme.error,
            'Pago rechazado',
            'No se registrará la venta. El carrito se conserva.',
          ),
          _ => (
            Icons.block,
            scheme.outline,
            'Pago cancelado',
            'No se registrará la venta. El carrito se conserva.',
          ),
        };
    return [
      Icon(icon, size: 54, color: color),
      const SizedBox(height: 8),
      Text(
        titulo,
        textAlign: TextAlign.center,
        style: theme.textTheme.titleLarge?.copyWith(
          fontWeight: FontWeight.w700,
          color: color,
        ),
      ),
      const SizedBox(height: 4),
      Text(
        detalle,
        textAlign: TextAlign.center,
        style: theme.textTheme.bodySmall?.copyWith(
          color: scheme.onSurfaceVariant,
        ),
      ),
      const SizedBox(height: 14),
      Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: scheme.surfaceContainerHighest.withValues(alpha: 0.5),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Column(
          children: [
            _dato(context, 'Autorización', resultado.autorizacion),
            _dato(context, 'Fecha', _fechaCorta(resultado.fecha)),
            _dato(context, 'Total', dinero(widget.total)),
          ],
        ),
      ),
    ];
  }

  Widget _dato(BuildContext context, String etiqueta, String valor) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: [
          Text(
            etiqueta,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const Spacer(),
          Text(valor, style: const TextStyle(fontWeight: FontWeight.w600)),
        ],
      ),
    );
  }
}

class _Tarjeta extends StatelessWidget {
  const _Tarjeta({required this.total});
  final int total;

  @override
  Widget build(BuildContext context) => AspectRatio(
    aspectRatio: 1.8,
    child: Container(
      padding: const EdgeInsets.all(18),
      decoration: BoxDecoration(
        gradient: const LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [Color(0xff0f3d2e), Color(0xff177256)],
        ),
        borderRadius: BorderRadius.circular(18),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Row(
            children: [
              Text(
                'MercaPOS',
                style: TextStyle(
                  color: Colors.white,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 0.4,
                ),
              ),
              Spacer(),
              Icon(Icons.contactless, color: Colors.white70),
            ],
          ),
          const Spacer(),
          const Icon(Icons.memory, size: 30, color: Color(0xffd9b25f)),
          const SizedBox(height: 8),
          const Text(
            '••••  ••••  ••••  4242',
            style: TextStyle(
              color: Colors.white,
              fontSize: 17,
              letterSpacing: 2,
            ),
          ),
          const Spacer(),
          Row(
            children: [
              const Text(
                'Visa / Mastercard',
                style: TextStyle(color: Colors.white70, fontSize: 11),
              ),
              const Spacer(),
              Text(
                dinero(total),
                style: const TextStyle(
                  color: Colors.white,
                  fontWeight: FontWeight.w700,
                  fontSize: 16,
                ),
              ),
            ],
          ),
        ],
      ),
    ),
  );
}

Future<void> mostrarComprobante(
  BuildContext context, {
  required String folio,
  required DateTime fecha,
  required List<LineaComprobante> lineas,
  required int total,
  required String autorizacion,
}) {
  return showDialog<void>(
    context: context,
    builder: (dialog) => AlertDialog(
      title: const Text('Comprobante de venta'),
      content: SizedBox(
        width: 380,
        child: SingleChildScrollView(
          child: _Comprobante(
            folio: folio,
            fecha: fecha,
            lineas: lineas,
            total: total,
            autorizacion: autorizacion,
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(dialog),
          child: const Text('Cerrar'),
        ),
        FilledButton.icon(
          onPressed: () async {
            final ok = await imprimirComprobante(
              _htmlComprobante(
                folio: folio,
                fecha: fecha,
                lineas: lineas,
                total: total,
                autorizacion: autorizacion,
              ),
            );
            if (!ok && dialog.mounted) {
              ScaffoldMessenger.of(dialog).showSnackBar(
                const SnackBar(
                  content: Text(
                    'No se pudo abrir la impresión. Permite ventanas emergentes.',
                  ),
                ),
              );
            }
          },
          icon: const Icon(Icons.print_outlined),
          label: const Text('Imprimir'),
        ),
      ],
    ),
  );
}

class _Comprobante extends StatelessWidget {
  const _Comprobante({
    required this.folio,
    required this.fecha,
    required this.lineas,
    required this.total,
    required this.autorizacion,
  });

  final String folio;
  final DateTime fecha;
  final List<LineaComprobante> lineas;
  final int total;
  final String autorizacion;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    const mono = TextStyle(fontFamily: 'monospace', fontSize: 12.5);
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(
          alpha: 0.35,
        ),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Center(
            child: Text(
              'MercaPOS',
              style: TextStyle(fontWeight: FontWeight.w800, fontSize: 18),
            ),
          ),
          const Center(child: Text('Comprobante de venta', style: mono)),
          const SizedBox(height: 10),
          Text('Folio: $folio', style: mono),
          Text('Fecha: ${_fechaCorta(fecha)}', style: mono),
          const Divider(height: 18),
          for (final linea in lineas)
            Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(linea.nombre, style: mono),
                        Text(
                          '${linea.cantidad} ${linea.unidad}',
                          style: mono.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                  Text(linea.importe, style: mono),
                ],
              ),
            ),
          const Divider(height: 18),
          Row(
            children: [
              const Text(
                'TOTAL',
                style: TextStyle(fontWeight: FontWeight.w800),
              ),
              const Spacer(),
              Text(
                dinero(total),
                style: const TextStyle(
                  fontWeight: FontWeight.w800,
                  fontSize: 16,
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),
          Text('Pago: tarjeta', style: mono),
          Text('Autorización: $autorizacion', style: mono),
          const SizedBox(height: 10),
          Center(
            child: Text(
              'Gracias por su compra.',
              style: mono.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
                fontSize: 11,
              ),
              textAlign: TextAlign.center,
            ),
          ),
        ],
      ),
    );
  }
}

String _htmlComprobante({
  required String folio,
  required DateTime fecha,
  required List<LineaComprobante> lineas,
  required int total,
  required String autorizacion,
}) {
  const escape = HtmlEscape();
  final filas = lineas
      .map(
        (linea) => '<tr><td>${escape.convert(linea.nombre)}<br><small>'
            '${escape.convert(linea.cantidad)} '
            '${escape.convert(linea.unidad)}</small></td>'
            '<td class="right">${escape.convert(linea.importe)}</td></tr>',
      )
      .join();
  return '''<!doctype html>
<html lang="es"><head><meta charset="utf-8">
<title>Comprobante $folio</title>
<style>
  body { font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 13px; color: #111; margin: 24px; }
  h1 { font-size: 18px; text-align: center; margin: 0 0 2px; }
  .sub { text-align: center; margin: 0 0 16px; }
  table { width: 100%; border-collapse: collapse; }
  td { padding: 4px 0; vertical-align: top; }
  .right { text-align: right; white-space: nowrap; }
  hr { border: 0; border-top: 1px dashed #888; margin: 12px 0; }
  .total { font-weight: 700; font-size: 15px; }
  .foot { text-align: center; color: #666; font-size: 11px; margin-top: 16px; }
  @page { margin: 12mm; }
</style></head>
<body onload="setTimeout(function(){window.print();}, 250)">
  <h1>MercaPOS</h1>
  <p class="sub">Comprobante de venta</p>
  <div>Folio: ${escape.convert(folio)}</div>
  <div>Fecha: ${escape.convert(_fechaCorta(fecha))}</div>
  <hr>
  <table>$filas</table>
  <hr>
  <table>
    <tr class="total"><td>TOTAL</td><td class="right">${escape.convert(dinero(total))}</td></tr>
  </table>
  <hr>
  <div>Pago: tarjeta</div>
  <div>Autorización: ${escape.convert(autorizacion)}</div>
  <p class="foot">Gracias por su compra.</p>
</body></html>''';
}
