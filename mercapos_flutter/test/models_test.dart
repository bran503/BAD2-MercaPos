import 'package:flutter_test/flutter_test.dart';
import 'package:mercapos_flutter/models.dart';

void main() {
  test('importe exacto y redondeo de cada renglón', () {
    expect(subtotalCentavos(parseScaled('2', 3), parseScaled('1.25', 4)), 250);
    expect(subtotalCentavos(parseScaled('1', 3), parseScaled('1.005', 4)), 101);
    expect(
      subtotalCentavos(parseScaled('0.125', 3), parseScaled('1.23', 4)),
      15,
    );
    expect(dinero(250), '\$2.50');
    expect(scaledText(-500, 3), '-0.5');
  });

  test('cantidades con más de tres decimales se rechazan', () {
    expect(() => parseScaled('0.0001', 3), throwsFormatException);
    expect(() => parseScaled('-1', 3), throwsFormatException);
  });
}
