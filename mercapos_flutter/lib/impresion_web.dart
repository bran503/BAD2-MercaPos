import 'dart:js_interop';

import 'package:web/web.dart' as web;

Future<bool> imprimirComprobante(String html) async {
  final blob = web.Blob(
    [html.toJS].toJS,
    web.BlobPropertyBag(type: 'text/html;charset=utf-8'),
  );
  final url = web.URL.createObjectURL(blob);
  final ventana = web.window.open(url, '_blank');
  return ventana != null;
}
