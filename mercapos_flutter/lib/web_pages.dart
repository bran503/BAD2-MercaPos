import 'package:flutter/material.dart';

import 'models.dart';
import 'pos_controller.dart';
import 'theme.dart';

String _s(dynamic value) => value?.toString() ?? '';
String _name(Map<String, dynamic> row) =>
    _s(row['nombre'] ?? row['nombre_producto'] ?? row['id']);

void _notice(BuildContext context, Object message) {
  if (context.mounted) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text('$message')));
  }
}

Future<bool> _perform(
  BuildContext context,
  Future<void> Function() action,
  String success,
) async {
  try {
    await action();
    if (context.mounted) _notice(context, success);
    return true;
  } catch (e) {
    if (context.mounted) _notice(context, e);
    return false;
  }
}

class _RemoteList extends StatefulWidget {
  const _RemoteList({required this.load, required this.builder});
  final Future<List<Map<String, dynamic>>> Function() load;
  final Widget Function(BuildContext, List<Map<String, dynamic>>, VoidCallback)
  builder;

  @override
  State<_RemoteList> createState() => _RemoteListState();
}

class _RemoteListState extends State<_RemoteList> {
  late Future<List<Map<String, dynamic>>> future;
  @override
  void initState() {
    super.initState();
    future = widget.load();
  }

  void reload() => setState(() => future = widget.load());

  @override
  Widget build(BuildContext context) =>
      FutureBuilder<List<Map<String, dynamic>>>(
        future: future,
        builder: (context, snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const _LoadingList();
          }
          if (snapshot.hasError) {
            final theme = Theme.of(context);
            return Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Card(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          Icons.cloud_off_outlined,
                          size: 36,
                          color: theme.colorScheme.error,
                        ),
                        const SizedBox(height: 12),
                        Text(
                          'No se pudieron cargar los datos',
                          style: theme.textTheme.titleMedium,
                        ),
                        const SizedBox(height: 6),
                        Text('${snapshot.error}', textAlign: TextAlign.center),
                        const SizedBox(height: 16),
                        FilledButton.tonalIcon(
                          onPressed: reload,
                          icon: const Icon(Icons.refresh),
                          label: const Text('Reintentar'),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            );
          }
          return widget.builder(context, snapshot.data ?? [], reload);
        },
      );
}

class _LoadingList extends StatelessWidget {
  const _LoadingList();

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(
      context,
    ).colorScheme.surfaceContainerHighest.withValues(alpha: 0.6);
    return ListView(
      padding: const EdgeInsets.fromLTRB(20, 24, 20, 20),
      children: [
        for (var i = 0; i < 6; i++)
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: Container(
              height: 76,
              decoration: BoxDecoration(
                color: color,
                borderRadius: BorderRadius.circular(16),
              ),
            ),
          ),
      ],
    );
  }
}

Widget _iconAvatar(
  BuildContext context,
  IconData icon, {
  Color? background,
  Color? foreground,
}) {
  final scheme = Theme.of(context).colorScheme;
  return CircleAvatar(
    backgroundColor: background ?? scheme.primaryContainer,
    foregroundColor: foreground ?? scheme.onPrimaryContainer,
    child: Icon(icon, size: 20),
  );
}

Widget _tileCard({
  required Widget leading,
  required String title,
  required String subtitle,
  Widget? trailing,
  VoidCallback? onTap,
}) => Padding(
  padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 4),
  child: Card(
    child: ListTile(
      leading: leading,
      title: Text(title, maxLines: 2, overflow: TextOverflow.ellipsis),
      subtitle: Text(subtitle, maxLines: 2, overflow: TextOverflow.ellipsis),
      trailing: trailing,
      onTap: onTap,
    ),
  ),
);

Widget _empty({
  required IconData icon,
  required String title,
  String? message,
  Widget? action,
}) => EmptyState(icon: icon, title: title, message: message, action: action);

Widget _heading(
  BuildContext context,
  String title,
  VoidCallback refresh, {
  VoidCallback? add,
  String addLabel = 'Agregar',
  String? subtitle,
}) => SectionHeader(
  title: title,
  subtitle: subtitle,
  actions: [
    IconButton.filledTonal(
      tooltip: 'Actualizar',
      onPressed: refresh,
      icon: const Icon(Icons.refresh),
    ),
    if (add != null)
      FilledButton.icon(
        onPressed: add,
        icon: const Icon(Icons.add),
        label: Text(addLabel),
      ),
  ],
);

class ProductsPage extends StatelessWidget {
  const ProductsPage({super.key, required this.controller});
  final PosController controller;

  Future<void> _edit(
    BuildContext context,
    VoidCallback reload, [
    Map<String, dynamic>? product,
  ]) async {
    final current = product == null
        ? null
        : await controller.obtenerProducto(_s(product['id']));
    if (!context.mounted) return;
    final name = TextEditingController(text: _s(current?['nombre']));
    final category = TextEditingController(text: _s(current?['categoria']));
    final unit = TextEditingController(text: _s(current?['unidad']));
    final price = TextEditingController(text: _s(current?['precio_unitario']));
    var active = current?['activo'] != false;
    await showDialog<void>(
      context: context,
      builder: (dialog) => StatefulBuilder(
        builder: (dialog, setDialog) => AlertDialog(
          title: Text(current == null ? 'Nuevo producto' : 'Editar producto'),
          content: SizedBox(
            width: 420,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  TextField(
                    controller: name,
                    decoration: const InputDecoration(labelText: 'Nombre'),
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: category,
                    decoration: const InputDecoration(labelText: 'Categoría'),
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: unit,
                    decoration: const InputDecoration(
                      labelText: 'Unidad (ej. libra)',
                    ),
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: price,
                    keyboardType: const TextInputType.numberWithOptions(
                      decimal: true,
                    ),
                    decoration: const InputDecoration(
                      labelText: 'Precio unitario',
                    ),
                  ),
                  if (current != null)
                    SwitchListTile(
                      title: const Text('Activo'),
                      value: active,
                      onChanged: (v) => setDialog(() => active = v),
                    ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialog),
              child: const Text('Cancelar'),
            ),
            FilledButton(
              onPressed: () async {
                if (name.text.trim().isEmpty ||
                    category.text.trim().isEmpty ||
                    unit.text.trim().isEmpty ||
                    price.text.trim().isEmpty) {
                  _notice(dialog, 'Completa todos los campos');
                  return;
                }
                final ok = await _perform(
                  dialog,
                  () => current == null
                      ? controller.crearProducto(
                          nombre: name.text,
                          categoria: category.text,
                          unidad: unit.text,
                          precio: price.text,
                        )
                      : controller.actualizarProducto(
                          current,
                          nombre: name.text,
                          categoria: category.text,
                          unidad: unit.text,
                          precio: price.text,
                          activo: active,
                        ),
                  'Producto guardado',
                );
                if (ok && dialog.mounted) {
                  Navigator.pop(dialog);
                  reload();
                }
              },
              child: const Text('Guardar'),
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) => _RemoteList(
    load: controller.listarProductos,
    builder: (context, rows, reload) => ListView(
      children: [
        _heading(
          context,
          'Productos (${rows.length})',
          reload,
          add: controller.esAdministrador ? () => _edit(context, reload) : null,
          addLabel: 'Nuevo producto',
          subtitle: 'Catálogo del puesto y precios de venta',
        ),
        if (rows.isEmpty)
          _empty(
            icon: Icons.inventory_2_outlined,
            title: 'Sin productos',
            message: controller.esAdministrador
                ? 'Crea el primer producto para empezar a vender.'
                : 'El administrador del puesto aún no ha creado productos.',
          ),
        for (final p in rows)
          _tileCard(
            leading: _iconAvatar(context, Icons.inventory_2_outlined),
            title: _name(p),
            subtitle: '${_s(p['categoria'])} · ${_s(p['unidad'])}',
            trailing: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                Text(
                  '\$${_s(p['precio_unitario'])}',
                  style: const TextStyle(fontWeight: FontWeight.w700),
                ),
                Text(
                  p['activo'] == false ? 'Inactivo' : 'Activo',
                  style: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                    color: p['activo'] == false
                        ? Theme.of(context).colorScheme.error
                        : Theme.of(context).colorScheme.primary,
                  ),
                ),
              ],
            ),
            onTap: controller.esAdministrador
                ? () => _edit(context, reload, p)
                : null,
          ),
      ],
    ),
  );
}

class InventoryPage extends StatelessWidget {
  const InventoryPage({super.key, required this.controller});
  final PosController controller;

  Future<void> _opening(
    BuildContext context,
    VoidCallback reload,
    List<Map<String, dynamic>> products,
  ) async {
    final qty = TextEditingController();
    String? productId = products.isEmpty ? null : _s(products.first['id']);
    await showDialog<void>(
      context: context,
      builder: (dialog) => StatefulBuilder(
        builder: (dialog, setDialog) => AlertDialog(
          title: const Text('Inventario inicial'),
          content: SizedBox(
            width: 400,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                DropdownButtonFormField<String>(
                  initialValue: productId,
                  decoration: const InputDecoration(labelText: 'Producto'),
                  items: [
                    for (final p in products)
                      DropdownMenuItem(
                        value: _s(p['id']),
                        child: Text(_name(p)),
                      ),
                  ],
                  onChanged: (v) => setDialog(() => productId = v),
                ),
                const SizedBox(height: 10),
                TextField(
                  controller: qty,
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                  ),
                  decoration: const InputDecoration(
                    labelText: 'Cantidad inicial',
                  ),
                ),
                const SizedBox(height: 8),
                const Text(
                  'Cada producto admite una sola apertura de inventario.',
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialog),
              child: const Text('Cancelar'),
            ),
            FilledButton(
              onPressed: () async {
                final ok = await _perform(
                  dialog,
                  () => controller.abrirInventario(
                    productId!,
                    parseScaled(qty.text, 3),
                  ),
                  'Inventario inicial registrado',
                );
                if (ok && dialog.mounted) {
                  Navigator.pop(dialog);
                  reload();
                }
              },
              child: const Text('Registrar'),
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) => _RemoteList(
    load: controller.listarProductos,
    builder: (context, products, reloadProducts) => _RemoteList(
      load: controller.listarMovimientos,
      builder: (context, movements, reloadMovements) {
        final names = {for (final p in products) _s(p['id']): _name(p)};
        return ListView(
          children: [
            _heading(
              context,
              'Existencias',
              () async {
                await _perform(
                  context,
                  controller.actualizarExistencias,
                  'Existencias actualizadas',
                );
                reloadMovements();
              },
              add: controller.esAdministrador && products.isNotEmpty
                  ? () => _opening(context, () {
                      reloadProducts();
                      reloadMovements();
                    }, products)
                  : null,
              addLabel: 'Inventario inicial',
            ),
            if (products.isEmpty)
              _empty(
                icon: Icons.warehouse_outlined,
                title: 'Sin existencias',
                message: 'Crea productos y registra el inventario inicial.',
              ),
            for (final p in products.where((p) => p['activo'] != false))
              Builder(
                builder: (context) {
                  final stock = controller.existenciasMil[_s(p['id'])];
                  final scheme = Theme.of(context).colorScheme;
                  return _tileCard(
                    leading: _iconAvatar(context, Icons.warehouse_outlined),
                    title: _name(p),
                    subtitle: _s(p['unidad']),
                    trailing: Text(
                      stock == null ? 'Sin dato' : scaledText(stock, 3),
                      style: TextStyle(
                        fontWeight: FontWeight.w700,
                        color: stock == null
                            ? scheme.onSurfaceVariant
                            : stock < 0
                            ? scheme.error
                            : scheme.primary,
                      ),
                    ),
                  );
                },
              ),
            const Divider(),
            Padding(
              padding: const EdgeInsets.all(16),
              child: Text(
                'Movimientos (${movements.length})',
                style: Theme.of(context).textTheme.titleLarge,
              ),
            ),
            for (final m in movements.reversed)
              Builder(
                builder: (context) {
                  final cantidad = num.tryParse(_s(m['cantidad'])) ?? 0;
                  final scheme = Theme.of(context).colorScheme;
                  return _tileCard(
                    leading: _iconAvatar(
                      context,
                      cantidad >= 0
                          ? Icons.arrow_downward
                          : Icons.arrow_upward,
                    ),
                    title:
                        names[_s(m['producto_id'])] ?? _s(m['producto_id']),
                    subtitle:
                        '${_s(m['motivo'])} · ${_s(m['fecha_local'])}',
                    trailing: Text(
                      '${cantidad > 0 ? '+' : ''}${_s(m['cantidad'])}',
                      style: TextStyle(
                        fontWeight: FontWeight.w700,
                        color: cantidad >= 0 ? scheme.primary : scheme.error,
                      ),
                    ),
                  );
                },
              ),
          ],
        );
      },
    ),
  );
}

class OperationsPage extends StatelessWidget {
  const OperationsPage({super.key, required this.controller});
  final PosController controller;

  Future<void> _create(
    BuildContext context,
    VoidCallback reload,
    String type,
    List<Map<String, dynamic>> operations,
  ) async {
    final products = controller.productos;
    if (type != 'devolucion' && products.isEmpty) {
      _notice(context, 'Actualiza el catálogo primero');
      return;
    }
    await showDialog<void>(
      context: context,
      builder: (dialog) => _OperationDialog(
        controller: controller,
        type: type,
        operations: operations,
        onSaved: () {
          Navigator.pop(dialog);
          reload();
        },
      ),
    );
  }

  void _detail(BuildContext context, Map<String, dynamic> op) =>
      showDialog<void>(
        context: context,
        builder: (dialog) => AlertDialog(
          title: Text('${_s(op['tipo_operacion'])} · ${_s(op['id'])}'),
          content: SizedBox(
            width: 480,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Fecha: ${_s(op['fecha_local'])}'),
                  Text('Total: \$${_s(op['total'])}'),
                  if (op['motivo'] != null) Text('Motivo: ${_s(op['motivo'])}'),
                  for (final row in (op['renglones'] as List? ?? []))
                    ListTile(
                      title: Text(
                        _s(
                          (row as Map)['nombre_producto'] ?? row['producto_id'],
                        ),
                      ),
                      subtitle: Text('Cantidad: ${_s(row['cantidad'])}'),
                    ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialog),
              child: const Text('Cerrar'),
            ),
          ],
        ),
      );

  @override
  Widget build(BuildContext context) => _RemoteList(
    load: () async {
      try {
        return await controller.listarOperaciones();
      } catch (_) {
        return [];
      }
    },
    builder: (context, rows, reload) {
      final originIds = rows.map((r) => _s(r['id'])).toSet();
      final origins = [
        ...rows,
        ...controller.ventas
            .where((v) => v.estado != 'rechazada' && !originIds.contains(v.id))
            .map((v) => v.cuerpo),
      ];
      return ListView(
        children: [
          _heading(
            context,
            'Operaciones',
            reload,
            subtitle: 'Ventas, compras, mermas y devoluciones',
          ),
          if (rows.isEmpty && controller.ventas.isEmpty)
            _empty(
              icon: Icons.receipt_long_outlined,
              title: 'Sin operaciones',
              message:
                  'Las ventas y movimientos del puesto aparecerán aquí.',
            ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                FilledButton.icon(
                  onPressed: () => _create(context, reload, 'compra', origins),
                  icon: const Icon(Icons.add_shopping_cart),
                  label: const Text('Compra'),
                ),
                FilledButton.icon(
                  onPressed: () => _create(context, reload, 'merma', origins),
                  icon: const Icon(Icons.remove_circle_outline),
                  label: const Text('Merma'),
                ),
                FilledButton.icon(
                  onPressed:
                      origins.any(
                        (x) =>
                            x['tipo_operacion'] == 'venta' ||
                            x['tipo_operacion'] == 'compra',
                      )
                      ? () => _create(context, reload, 'devolucion', origins)
                      : null,
                  icon: const Icon(Icons.assignment_return_outlined),
                  label: const Text('Devolución'),
                ),
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Text(
              'En este dispositivo',
              style: Theme.of(context).textTheme.titleMedium,
            ),
          ),
          for (final op in controller.ventas.reversed)
            _tileCard(
              leading: _iconAvatar(context, Icons.receipt_long_outlined),
              title:
                  '${_s(op.cuerpo['tipo_operacion'])} · ${op.estado.replaceAll('_', ' ')}',
              subtitle: '${op.fecha} · ${op.mensaje ?? op.id}',
              trailing: op.estado == 'rechazada'
                  ? IconButton(
                      tooltip: 'Reintentar',
                      icon: const Icon(Icons.replay),
                      onPressed: () => _perform(
                        context,
                        () => controller.reintentar(op),
                        'Reintento enviado',
                      ),
                    )
                  : null,
              onTap: () => _detail(context, op.cuerpo),
            ),
          const Divider(),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Text(
              'En el servidor (${rows.length})',
              style: Theme.of(context).textTheme.titleMedium,
            ),
          ),
          for (final op in rows.reversed)
            _tileCard(
              leading: _iconAvatar(context, Icons.cloud_done_outlined),
              title:
                  '${_s(op['tipo_operacion'])} · \$${_s(op['total'])}',
              subtitle: '${_s(op['fecha_local'])} · ${_s(op['id'])}',
              trailing: const Icon(Icons.chevron_right),
              onTap: () => _detail(context, op),
            ),
        ],
      );
    },
  );
}

class _OperationDialog extends StatefulWidget {
  const _OperationDialog({
    required this.controller,
    required this.type,
    required this.operations,
    required this.onSaved,
  });
  final PosController controller;
  final String type;
  final List<Map<String, dynamic>> operations;
  final VoidCallback onSaved;
  @override
  State<_OperationDialog> createState() => _OperationDialogState();
}

class _OperationDialogState extends State<_OperationDialog> {
  final qty = TextEditingController();
  final cost = TextEditingController();
  final reason = TextEditingController();
  final Map<String, Map<String, dynamic>> lines = {};
  String? productId;
  String? originId;
  String destination = 'vendible';
  bool saving = false;

  List<Map<String, dynamic>> get origins => widget.operations
      .where(
        (o) =>
            o['tipo_operacion'] == 'venta' || o['tipo_operacion'] == 'compra',
      )
      .toList();
  Map<String, dynamic>? get origin =>
      origins.where((o) => o['id'] == originId).firstOrNull;
  String get returnType =>
      origin?['tipo_operacion'] == 'compra' ? 'proveedor' : 'cliente';
  List<Producto> get selectable {
    if (widget.type != 'devolucion') return widget.controller.productos;
    return (origin?['renglones'] as List? ?? []).map((item) {
      final row = item as Map;
      return Producto(
        id: _s(row['producto_id']),
        puestoId: widget.controller.usuario!.puestoId,
        nombre: _s(row['nombre_producto'] ?? row['producto_id']),
        categoria: '',
        unidad: _s(row['unidad']),
        precio: (row['precio_unitario'] as num?) ?? 0,
        activo: true,
      );
    }).toList();
  }

  void addLine() {
    try {
      final id = productId ?? selectable.first.id;
      final amount = parseScaled(qty.text, 3);
      if (amount <= 0 || amount > 1000000000) {
        throw const FormatException('Cantidad fuera de límite');
      }
      final row = <String, dynamic>{
        'producto_id': id,
        'cantidad': num.parse(scaledText(amount, 3)),
      };
      if (widget.type == 'compra') {
        final value = parseScaled(cost.text, 4);
        if (value > 10000000000) {
          throw const FormatException('Costo fuera de límite');
        }
        row['costo_unitario'] = num.parse(scaledText(value, 4));
      } else if (widget.type == 'devolucion') {
        final original = (origin?['renglones'] as List)
            .cast<Map<String, dynamic>>()
            .firstWhere((r) => r['producto_id'] == id);
        row[returnType == 'cliente' ? 'precio_unitario' : 'costo_unitario'] =
            original[returnType == 'cliente'
                ? 'precio_unitario'
                : 'costo_unitario'];
      }
      setState(() {
        lines[id] = row;
        qty.clear();
        cost.clear();
      });
    } catch (e) {
      _notice(context, e);
    }
  }

  Future<void> save() async {
    if (saving || lines.isEmpty) {
      _notice(context, 'Agrega al menos un producto');
      return;
    }
    if (widget.type == 'merma' && reason.text.trim().isEmpty) {
      _notice(context, 'Indica el motivo');
      return;
    }
    if (widget.type == 'devolucion' && originId == null) {
      _notice(context, 'Selecciona la operación de origen');
      return;
    }
    setState(() => saving = true);
    final ok = await _perform(context, () async {
      final saved = await widget.controller.registrarOperacion(
        tipo: widget.type,
        renglones: lines.values.toList(),
        motivo: widget.type == 'merma' ? reason.text : null,
        origenId: widget.type == 'devolucion' ? originId : null,
        tipoDevolucion: widget.type == 'devolucion' ? returnType : null,
        destinoDevolucion:
            widget.type == 'devolucion' && destination == 'no_vendible'
            ? destination
            : null,
      );
      if (saved.estado == 'rechazada') {
        throw ApiError(saved.mensaje ?? 'Operación rechazada');
      }
    }, 'Operación guardada');
    if (mounted) setState(() => saving = false);
    if (ok && mounted) widget.onSaved();
  }

  @override
  Widget build(BuildContext context) {
    final options = selectable;
    return AlertDialog(
      title: Text(
        {
          'compra': 'Nueva compra',
          'merma': 'Registrar merma',
          'devolucion': 'Registrar devolución',
        }[widget.type]!,
      ),
      content: SizedBox(
        width: 520,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (widget.type == 'devolucion') ...[
                DropdownButtonFormField<String>(
                  initialValue: originId,
                  decoration: const InputDecoration(
                    labelText: 'Operación de origen',
                  ),
                  items: [
                    for (final o in origins)
                      DropdownMenuItem(
                        value: _s(o['id']),
                        child: Text(
                          '${_s(o['tipo_operacion'])} · ${_s(o['fecha_local'])}',
                        ),
                      ),
                  ],
                  onChanged: (v) => setState(() {
                    originId = v;
                    productId = null;
                    lines.clear();
                  }),
                ),
                const SizedBox(height: 10),
                if (originId != null) Text('Tipo: $returnType'),
                if (returnType == 'cliente' && originId != null)
                  DropdownButtonFormField<String>(
                    initialValue: destination,
                    decoration: const InputDecoration(labelText: 'Destino'),
                    items: const [
                      DropdownMenuItem(
                        value: 'vendible',
                        child: Text('Reingresa a inventario'),
                      ),
                      DropdownMenuItem(
                        value: 'no_vendible',
                        child: Text('No vendible'),
                      ),
                    ],
                    onChanged: (v) =>
                        setState(() => destination = v ?? 'vendible'),
                  ),
              ],
              if (options.isNotEmpty &&
                  (widget.type != 'devolucion' || originId != null)) ...[
                const SizedBox(height: 10),
                DropdownButtonFormField<String>(
                  initialValue: productId ?? options.first.id,
                  decoration: const InputDecoration(labelText: 'Producto'),
                  items: [
                    for (final p in options)
                      DropdownMenuItem(value: p.id, child: Text(p.nombre)),
                  ],
                  onChanged: (v) => setState(() => productId = v),
                ),
                const SizedBox(height: 10),
                TextField(
                  controller: qty,
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                  ),
                  decoration: const InputDecoration(labelText: 'Cantidad'),
                ),
                if (widget.type == 'compra') ...[
                  const SizedBox(height: 10),
                  TextField(
                    controller: cost,
                    keyboardType: const TextInputType.numberWithOptions(
                      decimal: true,
                    ),
                    decoration: const InputDecoration(
                      labelText: 'Costo unitario',
                    ),
                  ),
                ],
                const SizedBox(height: 8),
                OutlinedButton.icon(
                  onPressed: addLine,
                  icon: const Icon(Icons.add),
                  label: const Text('Agregar renglón'),
                ),
              ],
              if (widget.type == 'merma')
                TextField(
                  controller: reason,
                  maxLines: 2,
                  decoration: const InputDecoration(labelText: 'Motivo'),
                ),
              for (final row in lines.values)
                ListTile(
                  title: Text(
                    selectable
                            .where((p) => p.id == row['producto_id'])
                            .firstOrNull
                            ?.nombre ??
                        _s(row['producto_id']),
                  ),
                  subtitle: Text(
                    'Cantidad ${_s(row['cantidad'])}${row['costo_unitario'] == null ? '' : ' · Costo \$${_s(row['costo_unitario'])}'}',
                  ),
                  trailing: IconButton(
                    icon: const Icon(Icons.delete_outline),
                    onPressed: () =>
                        setState(() => lines.remove(row['producto_id'])),
                  ),
                ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancelar'),
        ),
        FilledButton(
          onPressed: saving ? null : save,
          child: Text(saving ? 'Guardando…' : 'Registrar'),
        ),
      ],
    );
  }
}

class CountsPage extends StatelessWidget {
  const CountsPage({super.key, required this.controller});
  final PosController controller;

  Future<void> _create(BuildContext context, VoidCallback reload) async {
    final products = await controller.listarProductos();
    if (!context.mounted) return;
    await showDialog<void>(
      context: context,
      builder: (dialog) => _CountDialog(
        controller: controller,
        products: products,
        onSaved: () {
          Navigator.pop(dialog);
          reload();
        },
      ),
    );
  }

  void _detail(
    BuildContext context,
    Map<String, dynamic> count,
    VoidCallback reload,
  ) => showDialog<void>(
    context: context,
    builder: (dialog) => AlertDialog(
      title: Text('Conteo ${_s(count['id'])}'),
      content: SizedBox(
        width: 500,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                '${_s(count['periodo_estudio'])} · ${_s(count['etapa'])} · ${_s(count['origen_registro'])}',
              ),
              if (count['evidencia'] != null)
                Text('Evidencia: ${_s(count['evidencia'])}'),
              for (final row in (count['renglones'] as List? ?? []))
                ListTile(
                  title: Text(_s((row as Map)['producto_id'])),
                  subtitle: Text(
                    'Contado ${_s(row['cantidad_contada'])} · Sistema ${_s(row['cantidad_sistema'])}',
                  ),
                  trailing: Text('Δ ${_s(row['diferencia'])}'),
                ),
            ],
          ),
        ),
      ),
      actions: [
        if (controller.esAdministrador &&
            count['origen_registro'] == 'pos' &&
            (count['renglones'] as List? ?? []).any(
              (r) => (r as Map)['diferencia'] != 0,
            ))
          FilledButton(
            onPressed: () async {
              final ok = await _perform(dialog, () async {
                final saved = await controller.ajustarConteo(count);
                if (saved.estado == 'rechazada') {
                  throw ApiError(saved.mensaje ?? 'Ajuste rechazado');
                }
              }, 'Ajuste registrado');
              if (ok && dialog.mounted) {
                Navigator.pop(dialog);
                reload();
              }
            },
            child: const Text('Aplicar ajuste'),
          ),
        TextButton(
          onPressed: () => Navigator.pop(dialog),
          child: const Text('Cerrar'),
        ),
      ],
    ),
  );

  @override
  Widget build(BuildContext context) => _RemoteList(
    load: controller.listarConteos,
    builder: (context, rows, reload) => ListView(
      children: [
        _heading(
          context,
          'Conteos físicos (${rows.length})',
          reload,
          add: () => _create(context, reload),
          addLabel: 'Nuevo conteo',
        ),
        if (rows.isEmpty)
          _empty(
            icon: Icons.fact_check_outlined,
            title: 'Sin conteos',
            message: 'Registra un conteo físico para comparar existencias.',
          ),
        for (final count in rows.reversed)
          _tileCard(
            leading: _iconAvatar(context, Icons.fact_check_outlined),
            title:
                '${_s(count['periodo_estudio'])} · ${_s(count['etapa'])}',
            subtitle:
                '${_s(count['origen_registro'])} · ${_s(count['fecha_local'])}',
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _detail(context, count, reload),
          ),
      ],
    ),
  );
}

class _CountDialog extends StatefulWidget {
  const _CountDialog({
    required this.controller,
    required this.products,
    required this.onSaved,
  });
  final PosController controller;
  final List<Map<String, dynamic>> products;
  final VoidCallback onSaved;
  @override
  State<_CountDialog> createState() => _CountDialogState();
}

class _CountDialogState extends State<_CountDialog> {
  final period = TextEditingController();
  final evidence = TextEditingController();
  final counted = TextEditingController();
  final system = TextEditingController();
  final Map<String, Map<String, dynamic>> lines = {};
  String stage = 'despues';
  String source = 'pos';
  String? productId;
  bool saving = false;

  void addLine() {
    try {
      final id = productId ?? _s(widget.products.first['id']);
      final amount = parseScaled(counted.text, 3);
      if (amount > 1000000000) {
        throw const FormatException('Cantidad fuera de límite');
      }
      final row = <String, dynamic>{
        'producto_id': id,
        'cantidad_contada': num.parse(scaledText(amount, 3)),
      };
      if (source == 'cuaderno') {
        final expected = parseScaled(system.text, 3);
        if (expected > 1000000000) {
          throw const FormatException('Cantidad de sistema fuera de límite');
        }
        row['cantidad_sistema'] = num.parse(scaledText(expected, 3));
      }
      setState(() {
        lines[id] = row;
        counted.clear();
        system.clear();
      });
    } catch (e) {
      _notice(context, e);
    }
  }

  Future<void> save() async {
    if (saving || period.text.trim().isEmpty || lines.isEmpty) {
      _notice(context, 'Indica período y productos contados');
      return;
    }
    if (source == 'cuaderno' && evidence.text.trim().isEmpty) {
      _notice(context, 'Indica la evidencia del cuaderno');
      return;
    }
    setState(() => saving = true);
    final ok = await _perform(context, () async {
      await widget.controller.crearConteo(
        periodo: period.text,
        etapa: stage,
        origen: source,
        renglones: lines.values.toList(),
        evidencia: source == 'cuaderno' ? evidence.text : null,
      );
    }, 'Conteo guardado');
    if (mounted) setState(() => saving = false);
    if (ok && mounted) widget.onSaved();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Nuevo conteo físico'),
    content: SizedBox(
      width: 520,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: period,
              decoration: const InputDecoration(
                labelText: 'Período de estudio',
              ),
            ),
            const SizedBox(height: 10),
            DropdownButtonFormField<String>(
              initialValue: stage,
              decoration: const InputDecoration(labelText: 'Etapa'),
              items: const [
                DropdownMenuItem(value: 'antes', child: Text('Antes')),
                DropdownMenuItem(value: 'despues', child: Text('Después')),
              ],
              onChanged: (v) => setState(() => stage = v ?? 'despues'),
            ),
            const SizedBox(height: 10),
            DropdownButtonFormField<String>(
              initialValue: source,
              decoration: const InputDecoration(labelText: 'Origen'),
              items: const [
                DropdownMenuItem(value: 'pos', child: Text('POS')),
                DropdownMenuItem(value: 'cuaderno', child: Text('Cuaderno')),
              ],
              onChanged: (v) => setState(() {
                source = v ?? 'pos';
                lines.clear();
              }),
            ),
            if (source == 'cuaderno') ...[
              const SizedBox(height: 10),
              TextField(
                controller: evidence,
                decoration: const InputDecoration(
                  labelText: 'Evidencia o referencia',
                ),
              ),
            ],
            const SizedBox(height: 10),
            if (widget.products.isNotEmpty) ...[
              DropdownButtonFormField<String>(
                initialValue: productId ?? _s(widget.products.first['id']),
                decoration: const InputDecoration(labelText: 'Producto'),
                items: [
                  for (final p in widget.products)
                    DropdownMenuItem(value: _s(p['id']), child: Text(_name(p))),
                ],
                onChanged: (v) => setState(() => productId = v),
              ),
              const SizedBox(height: 10),
              TextField(
                controller: counted,
                keyboardType: const TextInputType.numberWithOptions(
                  decimal: true,
                ),
                decoration: const InputDecoration(
                  labelText: 'Cantidad contada',
                ),
              ),
              if (source == 'cuaderno') ...[
                const SizedBox(height: 10),
                TextField(
                  controller: system,
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                  ),
                  decoration: const InputDecoration(
                    labelText: 'Cantidad en sistema',
                  ),
                ),
              ],
              const SizedBox(height: 8),
              OutlinedButton.icon(
                onPressed: addLine,
                icon: const Icon(Icons.add),
                label: const Text('Agregar producto'),
              ),
            ],
            for (final row in lines.values)
              ListTile(
                title: Text(
                  _name(
                    widget.products.firstWhere(
                      (p) => p['id'] == row['producto_id'],
                    ),
                  ),
                ),
                subtitle: Text(
                  'Contado: ${_s(row['cantidad_contada'])}${source == 'cuaderno' ? ' · Sistema: ${_s(row['cantidad_sistema'])}' : ''}',
                ),
                trailing: IconButton(
                  icon: const Icon(Icons.delete_outline),
                  onPressed: () =>
                      setState(() => lines.remove(row['producto_id'])),
                ),
              ),
          ],
        ),
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('Cancelar'),
      ),
      FilledButton(
        onPressed: saving ? null : save,
        child: const Text('Guardar conteo'),
      ),
    ],
  );
}

class UsersPage extends StatelessWidget {
  const UsersPage({super.key, required this.controller});
  final PosController controller;

  Future<void> _edit(
    BuildContext context,
    VoidCallback reload, [
    Map<String, dynamic>? row,
  ]) async {
    final id = TextEditingController(
      text: _s(row?['id']).replaceFirst('usuario:', ''),
    );
    final name = TextEditingController(text: _s(row?['nombre']));
    final password = TextEditingController();
    var role = _s(row?['rol']).isEmpty ? 'vendedor' : _s(row?['rol']);
    var active = row?['activo'] != false;
    await showDialog<void>(
      context: context,
      builder: (dialog) => StatefulBuilder(
        builder: (dialog, setDialog) => AlertDialog(
          title: Text(row == null ? 'Nuevo usuario' : 'Editar usuario'),
          content: SizedBox(
            width: 420,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  TextField(
                    controller: id,
                    enabled: row == null,
                    decoration: const InputDecoration(
                      labelText: 'Identificador',
                    ),
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: name,
                    decoration: const InputDecoration(labelText: 'Nombre'),
                  ),
                  const SizedBox(height: 10),
                  DropdownButtonFormField<String>(
                    initialValue: role,
                    decoration: const InputDecoration(labelText: 'Rol'),
                    items: const [
                      DropdownMenuItem(
                        value: 'vendedor',
                        child: Text('Vendedor'),
                      ),
                      DropdownMenuItem(
                        value: 'administrador',
                        child: Text('Administrador'),
                      ),
                    ],
                    onChanged: (v) => setDialog(() => role = v ?? 'vendedor'),
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: password,
                    obscureText: true,
                    decoration: InputDecoration(
                      labelText: row == null
                          ? 'Contraseña (mínimo 8 caracteres)'
                          : 'Nueva contraseña (opcional)',
                    ),
                  ),
                  if (row != null)
                    SwitchListTile(
                      title: const Text('Activo'),
                      value: active,
                      onChanged: (v) => setDialog(() => active = v),
                    ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialog),
              child: const Text('Cancelar'),
            ),
            FilledButton(
              onPressed: () async {
                if (id.text.trim().isEmpty ||
                    name.text.trim().isEmpty ||
                    (row == null && password.text.length < 8) ||
                    (password.text.isNotEmpty && password.text.length < 8)) {
                  _notice(
                    dialog,
                    'Completa los campos; la contraseña debe tener al menos 8 caracteres',
                  );
                  return;
                }
                final ok = await _perform(
                  dialog,
                  () => row == null
                      ? controller.crearUsuario(
                          id: id.text,
                          nombre: name.text,
                          rol: role,
                          password: password.text,
                        )
                      : controller.actualizarUsuario(
                          row,
                          nombre: name.text,
                          rol: role,
                          activo: active,
                          password: password.text,
                        ),
                  'Usuario guardado',
                );
                if (ok && dialog.mounted) {
                  Navigator.pop(dialog);
                  reload();
                }
              },
              child: const Text('Guardar'),
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) => _RemoteList(
    load: controller.listarUsuarios,
    builder: (context, rows, reload) => ListView(
      children: [
        _heading(
          context,
          'Usuarios (${rows.length})',
          reload,
          add: () => _edit(context, reload),
          addLabel: 'Nuevo usuario',
        ),
        if (rows.isEmpty)
          _empty(
            icon: Icons.people_outline,
            title: 'Sin usuarios',
            message: 'Agrega vendedores o administradores del puesto.',
          ),
        for (final row in rows)
          _tileCard(
            leading: _iconAvatar(context, Icons.person_outline),
            title: _name(row),
            subtitle: '${_s(row['id'])} · ${_s(row['rol'])}',
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  row['activo'] == false ? 'Inactivo' : 'Activo',
                  style: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                    color: row['activo'] == false
                        ? Theme.of(context).colorScheme.error
                        : Theme.of(context).colorScheme.primary,
                  ),
                ),
                const SizedBox(width: 6),
                const Icon(Icons.edit_outlined),
              ],
            ),
            onTap: () => _edit(context, reload, row),
          ),
      ],
    ),
  );
}

class IncidentsPage extends StatelessWidget {
  const IncidentsPage({super.key, required this.controller});
  final PosController controller;

  Future<void> _resolve(
    BuildContext context,
    VoidCallback reload,
    Map<String, dynamic> incident,
  ) async {
    final note = TextEditingController();
    await showDialog<void>(
      context: context,
      builder: (dialog) => AlertDialog(
        title: const Text('Resolver incidencia'),
        content: SizedBox(
          width: 400,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(_s(incident['mensaje'] ?? incident['motivo'])),
              const SizedBox(height: 10),
              TextField(
                controller: note,
                maxLines: 3,
                decoration: const InputDecoration(
                  labelText: 'Nota de resolución',
                ),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialog),
            child: const Text('Cancelar'),
          ),
          FilledButton(
            onPressed: () async {
              final ok = await _perform(
                dialog,
                () => controller.resolverIncidencia(
                  _s(incident['operacion_id']),
                  note.text,
                ),
                'Incidencia resuelta',
              );
              if (ok && dialog.mounted) {
                Navigator.pop(dialog);
                reload();
              }
            },
            child: const Text('Resolver'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) => _RemoteList(
    load: controller.listarIncidencias,
    builder: (context, rows, reload) => ListView(
      children: [
        _heading(context, 'Incidencias (${rows.length})', reload),
        if (rows.isEmpty)
          _empty(
            icon: Icons.check_circle_outline,
            title: 'Sin incidencias',
            message: 'Las operaciones con problemas aparecerán aquí.',
          ),
        for (final row in rows)
          _tileCard(
            leading: _iconAvatar(
              context,
              row['resuelta'] == true
                  ? Icons.check_circle_outline
                  : Icons.warning_amber,
              background: row['resuelta'] == true
                  ? Theme.of(context).colorScheme.secondaryContainer
                  : Theme.of(context).colorScheme.tertiaryContainer,
              foreground: row['resuelta'] == true
                  ? Theme.of(context).colorScheme.onSecondaryContainer
                  : Theme.of(context).colorScheme.onTertiaryContainer,
            ),
            title: _s(
              row['mensaje'] ?? row['motivo'] ?? row['operacion_id'],
            ),
            subtitle:
                '${_s(row['operacion_id'])} · ${row['resuelta'] == true ? 'Resuelta' : 'Pendiente'}',
            trailing: row['resuelta'] == true
                ? null
                : TextButton(
                    onPressed: () => _resolve(context, reload, row),
                    child: const Text('Resolver'),
                  ),
          ),
      ],
    ),
  );
}

class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key, required this.controller});
  final PosController controller;
  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  final current = TextEditingController();
  final next = TextEditingController();
  late Future<Map<String, dynamic>> puesto;

  @override
  void initState() {
    super.initState();
    puesto = widget.controller.obtenerPuesto();
  }

  @override
  void dispose() {
    current.dispose();
    next.dispose();
    super.dispose();
  }

  Future<void> _edit(Map<String, dynamic> row) async {
    final name = TextEditingController(text: _s(row['nombre']));
    final market = TextEditingController(text: _s(row['mercado']));
    final location = TextEditingController(text: _s(row['ubicacion']));
    var active = row['activo'] != false;
    await showDialog<void>(
      context: context,
      builder: (dialog) => StatefulBuilder(
        builder: (dialog, setDialog) => AlertDialog(
          title: const Text('Editar puesto'),
          content: SizedBox(
            width: 420,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  controller: name,
                  decoration: const InputDecoration(labelText: 'Nombre'),
                ),
                const SizedBox(height: 10),
                TextField(
                  controller: market,
                  decoration: const InputDecoration(labelText: 'Mercado'),
                ),
                const SizedBox(height: 10),
                TextField(
                  controller: location,
                  decoration: const InputDecoration(labelText: 'Ubicación'),
                ),
                SwitchListTile(
                  title: const Text('Activo'),
                  value: active,
                  onChanged: (v) => setDialog(() => active = v),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialog),
              child: const Text('Cancelar'),
            ),
            FilledButton(
              onPressed: () async {
                final ok = await _perform(
                  dialog,
                  () => widget.controller.actualizarPuesto(
                    row,
                    nombre: name.text,
                    mercado: market.text,
                    ubicacion: location.text,
                    activo: active,
                  ),
                  'Puesto guardado',
                );
                if (ok && dialog.mounted) {
                  Navigator.pop(dialog);
                  setState(() => puesto = widget.controller.obtenerPuesto());
                }
              },
              child: const Text('Guardar'),
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) => ListView(
    padding: const EdgeInsets.all(16),
    children: [
      Text('Perfil', style: Theme.of(context).textTheme.titleLarge),
      ListTile(
        title: Text(widget.controller.usuario!.nombre),
        subtitle: Text(
          '${widget.controller.usuario!.id} · ${widget.controller.usuario!.rol}',
        ),
      ),
      const SizedBox(height: 8),
      ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 420),
        child: Column(
          children: [
            TextField(
              controller: current,
              obscureText: true,
              decoration: const InputDecoration(labelText: 'Contraseña actual'),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: next,
              obscureText: true,
              decoration: const InputDecoration(
                labelText: 'Contraseña nueva (mínimo 8 caracteres)',
              ),
            ),
            const SizedBox(height: 10),
            FilledButton(
              onPressed: () async {
                if (next.text.length < 8) {
                  _notice(
                    context,
                    'La contraseña nueva debe tener al menos 8 caracteres',
                  );
                  return;
                }
                final ok = await _perform(
                  context,
                  () => widget.controller.cambiarPassword(
                    current.text,
                    next.text,
                  ),
                  'Contraseña cambiada',
                );
                if (ok) {
                  current.clear();
                  next.clear();
                }
              },
              child: const Text('Cambiar contraseña'),
            ),
          ],
        ),
      ),
      const Divider(height: 36),
      FutureBuilder<Map<String, dynamic>>(
        future: puesto,
        builder: (context, snapshot) {
          if (!snapshot.hasData) {
            return Text(
              snapshot.hasError ? '${snapshot.error}' : 'Cargando puesto…',
            );
          }
          final row = snapshot.data!;
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('Puesto', style: Theme.of(context).textTheme.titleLarge),
              ListTile(
                title: Text(_name(row)),
                subtitle: Text(
                  '${_s(row['mercado'])} · ${_s(row['ubicacion'])}',
                ),
              ),
              if (widget.controller.esAdministrador)
                OutlinedButton.icon(
                  onPressed: () => _edit(row),
                  icon: const Icon(Icons.edit_outlined),
                  label: const Text('Editar puesto'),
                ),
            ],
          );
        },
      ),
    ],
  );
}
