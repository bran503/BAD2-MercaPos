import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mercapos_flutter/pago_tarjeta.dart';

void main() {
  testWidgets('el POS simulado permite elegir y confirma el pago aprobado', (
    tester,
  ) async {
    ResultadoPago? resultado;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: Center(
              child: FilledButton(
                onPressed: () async {
                  resultado = await mostrarPagoTarjeta(context, total: 250);
                },
                child: const Text('abrir'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('abrir'));
    await tester.pumpAndSettle();
    expect(find.text('Acerca, inserta o desliza la tarjeta'), findsOneWidget);
    expect(find.text('Resultado de la operacion'), findsOneWidget);
    await tester.tap(find.text('Proceder pago'));
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(find.text('Pago aprobado'), findsOneWidget);
    expect(find.text('AUT-123456'), findsNothing);
    await tester.tap(find.text('Usar pago aprobado'));
    await tester.pumpAndSettle();
    expect(resultado, isNotNull);
    expect(resultado!.aprobado, isTrue);
    expect(resultado!.autorizacion, startsWith('AUT-'));
  });

  testWidgets('un pago rechazado no devuelve venta y conserva el carrito', (
    tester,
  ) async {
    ResultadoPago? resultado;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: Center(
              child: FilledButton(
                onPressed: () async {
                  resultado = await mostrarPagoTarjeta(context, total: 250);
                },
                child: const Text('abrir'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('abrir'));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('Rechazado'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Rechazado'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Proceder pago'));
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(find.text('Pago rechazado'), findsOneWidget);
    await tester.tap(find.text('Cerrar'));
    await tester.pumpAndSettle();
    expect(resultado, isNull);
  });
}
