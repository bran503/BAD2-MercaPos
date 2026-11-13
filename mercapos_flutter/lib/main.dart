import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'local_store.dart';
import 'models.dart';
import 'pago_tarjeta.dart';
import 'pos_controller.dart';
import 'theme.dart';
import 'web_pages.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final store = await LocalStore.open();
  final controller = PosController(store);
  await controller.iniciar();
  runApp(MercaPosApp(controller: controller));
}

class MercaPosApp extends StatelessWidget {
  const MercaPosApp({super.key, required this.controller});
  final PosController controller;

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'MercaPOS',
    debugShowCheckedModeBanner: false,
    theme: mercaTheme(),
    home: AnimatedBuilder(
      animation: controller,
      builder: (context, _) => controller.usuario == null
          ? LoginPage(controller: controller)
          : PosHome(controller: controller),
    ),
  );
}

class LoginPage extends StatefulWidget {
  const LoginPage({super.key, required this.controller});
  final PosController controller;

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  final url = TextEditingController();
  final user = TextEditingController();
  final password = TextEditingController();
  bool loading = false;
  bool mostrarPassword = false;
  String? error;

  @override
  void initState() {
    super.initState();
    url.text = widget.controller.baseUrl;
  }

  @override
  void dispose() {
    url.dispose();
    user.dispose();
    password.dispose();
    super.dispose();
  }

  Future<void> submit() async {
    setState(() {
      loading = true;
      error = null;
    });
    try {
      await widget.controller.login(url.text, user.text, password.text);
    } catch (e) {
      if (mounted) setState(() => error = e.toString());
    } finally {
      if (mounted) setState(() => loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      body: DecoratedBox(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: [
              theme.colorScheme.primary.withValues(alpha: 0.14),
              theme.scaffoldBackgroundColor,
              theme.colorScheme.secondaryContainer.withValues(alpha: 0.35),
            ],
          ),
        ),
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 440),
              child: Card(
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(28, 32, 28, 28),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Center(
                        child: Container(
                          padding: const EdgeInsets.all(16),
                          decoration: BoxDecoration(
                            color: theme.colorScheme.primary,
                            shape: BoxShape.circle,
                          ),
                          child: Icon(
                            Icons.storefront,
                            size: 34,
                            color: theme.colorScheme.onPrimary,
                          ),
                        ),
                      ),
                      const SizedBox(height: 18),
                      Text(
                        'MercaPOS',
                        textAlign: TextAlign.center,
                        style: theme.textTheme.headlineMedium?.copyWith(
                          fontWeight: FontWeight.w800,
                          letterSpacing: -0.5,
                        ),
                      ),
                      const SizedBox(height: 6),
                      Text(
                        'Punto de venta para tu puesto',
                        textAlign: TextAlign.center,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                      const SizedBox(height: 26),
                      TextField(
                        controller: user,
                        textInputAction: TextInputAction.next,
                        decoration: const InputDecoration(
                          labelText: 'Usuario',
                          prefixIcon: Icon(Icons.person_outline),
                        ),
                      ),
                      const SizedBox(height: 12),
                      TextField(
                        controller: password,
                        obscureText: !mostrarPassword,
                        onSubmitted: (_) => submit(),
                        decoration: InputDecoration(
                          labelText: 'Contraseña',
                          prefixIcon: const Icon(Icons.lock_outline),
                          suffixIcon: IconButton(
                            tooltip: mostrarPassword
                                ? 'Ocultar contraseña'
                                : 'Mostrar contraseña',
                            icon: Icon(
                              mostrarPassword
                                  ? Icons.visibility_off
                                  : Icons.visibility,
                            ),
                            onPressed: () => setState(
                              () => mostrarPassword = !mostrarPassword,
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(height: 8),
                      Theme(
                        data: theme.copyWith(dividerColor: Colors.transparent),
                        child: ExpansionTile(
                          tilePadding: EdgeInsets.zero,
                          childrenPadding: const EdgeInsets.only(bottom: 8),
                          leading: const Icon(Icons.dns_outlined, size: 20),
                          title: Text(
                            'Conexión',
                            style: theme.textTheme.labelLarge,
                          ),
                          subtitle: Text(
                            widget.controller.baseUrl,
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                          children: [
                            TextField(
                              controller: url,
                              keyboardType: TextInputType.url,
                              decoration: const InputDecoration(
                                labelText: 'Dirección de la API',
                                helperText: 'Ej. http://192.168.1.10:3000',
                              ),
                            ),
                          ],
                        ),
                      ),
                      if (error != null) ...[
                        const SizedBox(height: 12),
                        Container(
                          padding: const EdgeInsets.all(12),
                          decoration: BoxDecoration(
                            color: theme.colorScheme.errorContainer,
                            borderRadius: BorderRadius.circular(12),
                          ),
                          child: Row(
                            children: [
                              Icon(
                                Icons.error_outline,
                                size: 20,
                                color: theme.colorScheme.onErrorContainer,
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text(
                                  error!,
                                  style: TextStyle(
                                    color: theme.colorScheme.onErrorContainer,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                      ],
                      const SizedBox(height: 18),
                      FilledButton(
                        onPressed: loading ? null : submit,
                        child: loading
                            ? const SizedBox(
                                height: 20,
                                width: 20,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              )
                            : const Text('Iniciar sesión'),
                      ),
                      const SizedBox(height: 14),
                      Text(
                        kIsWeb
                            ? 'Funciona sin conexión después del primer inicio de sesión.'
                            : 'En emulador Android usa 10.0.2.2:3000. En un celular, usa la IP local de la computadora.',
                        textAlign: TextAlign.center,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class PosHome extends StatefulWidget {
  const PosHome({super.key, required this.controller});
  final PosController controller;

  @override
  State<PosHome> createState() => _PosHomeState();
}

class _PosHomeState extends State<PosHome> with WidgetsBindingObserver {
  int tab = 0;
  final sections = const [
    ('Vender', Icons.point_of_sale),
    ('Operaciones', Icons.receipt_long),
    ('Productos', Icons.inventory_2_outlined),
    ('Inventario', Icons.warehouse_outlined),
    ('Conteos', Icons.fact_check_outlined),
    ('Incidencias', Icons.report_problem_outlined),
    ('Usuarios', Icons.people_outline),
    ('Puesto y perfil', Icons.settings_outlined),
  ];

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) widget.controller.conectar();
  }

  Future<void> _action(Future<void> Function() action, String success) async {
    try {
      await action();
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(success)));
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(e.toString())));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final controller = widget.controller;
    final user = controller.usuario!;
    final pending = controller.ventas
        .where((v) => v.estado == 'pendiente')
        .length;
    final visible = controller.esAdministrador
        ? List<int>.generate(sections.length, (i) => i)
        : [0, 1, 2, 3, 4, 7];
    final wide = esAncho(context);
    final page = switch (tab) {
      0 => VentaPage(controller: controller),
      1 => OperationsPage(controller: controller),
      2 => ProductsPage(controller: controller),
      3 => InventoryPage(controller: controller),
      4 => CountsPage(controller: controller),
      5 => IncidentsPage(controller: controller),
      6 => UsersPage(controller: controller),
      _ => SettingsPage(controller: controller),
    };
    final body = Column(
      children: [
        if (controller.aviso != null)
          _AvisoBanner(
            mensaje: controller.aviso!,
            onRetry: () =>
                _action(controller.sincronizar, 'Sincronización terminada'),
          ),
        if (controller.sincronizando)
          const LinearProgressIndicator(minHeight: 3),
        Expanded(child: page),
      ],
    );
    final actions = [
      if (esMedio(context))
        _StatusChip(controller: controller, pending: pending),
      IconButton(
        tooltip: 'Actualizar catálogo',
        onPressed: () =>
            _action(controller.actualizarCatalogo, 'Catálogo actualizado'),
        icon: const Icon(Icons.sync_alt),
      ),
      IconButton(
        tooltip: 'Sincronizar',
        onPressed: controller.sincronizando
            ? null
            : () =>
                  _action(controller.sincronizar, 'Sincronización terminada'),
        icon: const Icon(Icons.cloud_upload_outlined),
      ),
      const SizedBox(width: 6),
    ];

    if (wide) {
      return Scaffold(
        body: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _SideNav(
              sections: sections,
              visible: visible,
              current: tab,
              pending: pending,
              user: user,
              onSelect: (i) => setState(() => tab = i),
              onLogout: () => _action(controller.logout, 'Sesión cerrada'),
            ),
            const VerticalDivider(width: 1),
            Expanded(
              child: Scaffold(
                appBar: AppBar(
                  title: Text(sections[tab].$1),
                  actions: actions,
                ),
                body: body,
              ),
            ),
          ],
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: Text('MercaPOS · ${sections[tab].$1}'),
        actions: actions,
      ),
      drawer: Drawer(
        child: _SideNav(
          sections: sections,
          visible: visible,
          current: tab,
          pending: pending,
          user: user,
          inDrawer: true,
          onSelect: (i) {
            Navigator.pop(context);
            setState(() => tab = i);
          },
          onLogout: () {
            Navigator.pop(context);
            _action(controller.logout, 'Sesión cerrada');
          },
        ),
      ),
      body: body,
    );
  }
}

class _StatusChip extends StatelessWidget {
  const _StatusChip({required this.controller, required this.pending});

  final PosController controller;
  final int pending;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final (IconData icon, String label, Color bg, Color fg) =
        controller.aviso != null
        ? (
            Icons.cloud_off_outlined,
            'Sin sincronizar',
            scheme.errorContainer,
            scheme.onErrorContainer,
          )
        : pending > 0
        ? (
            Icons.cloud_upload_outlined,
            '$pending pendiente${pending == 1 ? '' : 's'}',
            scheme.tertiaryContainer,
            scheme.onTertiaryContainer,
          )
        : (
            Icons.cloud_done_outlined,
            'Al día',
            scheme.primaryContainer,
            scheme.onPrimaryContainer,
          );
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 10),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        decoration: BoxDecoration(
          color: bg,
          borderRadius: BorderRadius.circular(999),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 16, color: fg),
            const SizedBox(width: 6),
            Text(
              label,
              style: TextStyle(
                color: fg,
                fontWeight: FontWeight.w600,
                fontSize: 12,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _AvisoBanner extends StatelessWidget {
  const _AvisoBanner({required this.mensaje, required this.onRetry});

  final String mensaje;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 12, 20, 0),
      child: Container(
        padding: const EdgeInsets.fromLTRB(14, 10, 8, 10),
        decoration: BoxDecoration(
          color: scheme.tertiaryContainer,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Row(
          children: [
            Icon(
              Icons.cloud_off_outlined,
              size: 20,
              color: scheme.onTertiaryContainer,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                mensaje,
                style: TextStyle(color: scheme.onTertiaryContainer),
              ),
            ),
            TextButton(onPressed: onRetry, child: const Text('Reintentar')),
          ],
        ),
      ),
    );
  }
}

class _SideNav extends StatelessWidget {
  const _SideNav({
    required this.sections,
    required this.visible,
    required this.current,
    required this.pending,
    required this.user,
    required this.onSelect,
    required this.onLogout,
    this.inDrawer = false,
  });

  final List<(String, IconData)> sections;
  final List<int> visible;
  final int current;
  final int pending;
  final Usuario user;
  final ValueChanged<int> onSelect;
  final VoidCallback onLogout;
  final bool inDrawer;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final nav = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 24, 20, 18),
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: scheme.primary,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Icon(
                  Icons.storefront,
                  color: scheme.onPrimary,
                  size: 22,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'MercaPOS',
                      style: theme.textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w800,
                      ),
                    ),
                    Text(
                      user.puestoId.replaceFirst('puesto:', ''),
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: scheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: ListView(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 10),
            children: [
              for (final i in visible)
                Padding(
                  padding: const EdgeInsets.only(bottom: 4),
                  child: Material(
                    color: current == i
                        ? scheme.primaryContainer
                        : Colors.transparent,
                    borderRadius: BorderRadius.circular(12),
                    child: InkWell(
                      borderRadius: BorderRadius.circular(12),
                      onTap: () => onSelect(i),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 11,
                        ),
                        child: Row(
                          children: [
                            Icon(
                              sections[i].$2,
                              size: 20,
                              color: current == i
                                  ? scheme.onPrimaryContainer
                                  : scheme.onSurfaceVariant,
                            ),
                            const SizedBox(width: 12),
                            Expanded(
                              child: Text(
                                sections[i].$1,
                                style: TextStyle(
                                  fontWeight: current == i
                                      ? FontWeight.w700
                                      : FontWeight.w500,
                                  color: current == i
                                      ? scheme.onPrimaryContainer
                                      : scheme.onSurface,
                                ),
                              ),
                            ),
                            if (i == 1 && pending > 0)
                              Badge(label: Text('$pending')),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
        const Divider(height: 1),
        ListTile(
          contentPadding: const EdgeInsets.symmetric(
            horizontal: 20,
            vertical: 6,
          ),
          leading: CircleAvatar(
            backgroundColor: scheme.secondaryContainer,
            child: Text(
              user.nombre.isEmpty
                  ? '?'
                  : user.nombre.substring(0, 1).toUpperCase(),
              style: TextStyle(
                color: scheme.onSecondaryContainer,
                fontWeight: FontWeight.w700,
              ),
            ),
          ),
          title: Text(user.nombre, overflow: TextOverflow.ellipsis),
          subtitle: Text(
            user.rol.replaceAll('_', ' '),
            overflow: TextOverflow.ellipsis,
          ),
          trailing: IconButton(
            tooltip: 'Cerrar sesión',
            onPressed: onLogout,
            icon: const Icon(Icons.logout),
          ),
        ),
      ],
    );
    if (inDrawer) return SafeArea(child: nav);
    return SafeArea(
      child: Container(color: scheme.surface, width: 254, child: nav),
    );
  }
}

class VentaPage extends StatefulWidget {
  const VentaPage({super.key, required this.controller});
  final PosController controller;

  @override
  State<VentaPage> createState() => _VentaPageState();
}

class _VentaPageState extends State<VentaPage> {
  final Map<String, int> carrito = {};
  final busqueda = TextEditingController();
  String filtro = '';
  String metodoPago = 'efectivo';
  bool guardando = false;

  @override
  void dispose() {
    busqueda.dispose();
    super.dispose();
  }

  void _agregar(Producto producto, [int delta = 1000]) {
    final actual = carrito[producto.id] ?? 0;
    final nuevo = (actual + delta).clamp(0, 1000000000);
    setState(() {
      if (nuevo <= 0) {
        carrito.remove(producto.id);
      } else {
        carrito[producto.id] = nuevo;
      }
    });
  }

  Future<void> cantidad(Producto producto) async {
    final input = TextEditingController(
      text: scaledText(carrito[producto.id] ?? 1000, 3),
    );
    final result = await showDialog<int>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(producto.nombre),
        content: TextField(
          controller: input,
          autofocus: true,
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          inputFormatters: [
            FilteringTextInputFormatter.allow(RegExp(r'[0-9.,]')),
          ],
          decoration: InputDecoration(
            labelText: 'Cantidad (${producto.unidad})',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancelar'),
          ),
          FilledButton(
            onPressed: () {
              try {
                final value = parseScaled(input.text, 3);
                if (value <= 0 || value > 1000000000) {
                  throw const FormatException('Cantidad fuera de límite');
                }
                Navigator.pop(context, value);
              } catch (e) {
                ScaffoldMessenger.of(
                  context,
                ).showSnackBar(SnackBar(content: Text(e.toString())));
              }
            },
            child: const Text('Agregar'),
          ),
        ],
      ),
    );
    input.dispose();
    if (result != null && mounted) {
      setState(() => carrito[producto.id] = result);
    }
  }

  Future<void> confirmar() async {
    if (carrito.isEmpty || guardando) return;
    final controller = widget.controller;
    final porId = {for (final p in controller.productos) p.id: p};
    var total = 0;
    for (final item in carrito.entries) {
      final producto = porId[item.key];
      if (producto != null) {
        total += subtotalCentavos(item.value, precioDiezmiles(producto.precio));
      }
    }
    final lineas = <LineaComprobante>[
      for (final item in carrito.entries)
        if (porId[item.key] != null)
          LineaComprobante(
            nombre: porId[item.key]!.nombre,
            cantidad: scaledText(item.value, 3),
            unidad: porId[item.key]!.unidad,
            importe: dinero(
              subtotalCentavos(
                item.value,
                precioDiezmiles(porId[item.key]!.precio),
              ),
            ),
          ),
    ];
    setState(() => guardando = true);
    try {
      ResultadoPago? pago;
      if (metodoPago == 'tarjeta') {
        pago = await mostrarPagoTarjeta(context, total: total);
        if (pago == null || !pago.aprobado) return;
      }
      final id = await controller.guardarVenta(carrito, pago: pago?.toJson());
      if (!mounted) return;
      setState(carrito.clear);
      if (pago != null) {
        await mostrarComprobante(
          context,
          folio: folioCorto(id),
          fecha: pago.fecha,
          lineas: lineas,
          total: total,
          autorizacion: pago.autorizacion,
        );
      } else {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Venta guardada en el dispositivo')),
        );
      }
      try {
        await controller.sincronizar();
      } catch (_) {
        // La venta ya se guardó y sigue visible como pendiente.
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(e.toString())));
      }
    } finally {
      if (mounted) setState(() => guardando = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final controller = widget.controller;
    final texto = filtro.toLowerCase();
    final productos = controller.productos
        .where(
          (p) =>
              p.nombre.toLowerCase().contains(texto) ||
              p.categoria.toLowerCase().contains(texto),
        )
        .toList();
    final porId = {for (final p in controller.productos) p.id: p};
    var total = 0;
    for (final item in carrito.entries) {
      final p = porId[item.key];
      if (p != null) {
        total += subtotalCentavos(item.value, precioDiezmiles(p.precio));
      }
    }

    final catalogo = Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 4, 20, 14),
          child: Row(
            children: [
              Expanded(
                child: TextField(
                  controller: busqueda,
                  onChanged: (value) => setState(() => filtro = value),
                  decoration: InputDecoration(
                    hintText: 'Buscar por nombre o categoría',
                    prefixIcon: const Icon(Icons.search),
                    suffixIcon: filtro.isEmpty
                        ? null
                        : IconButton(
                            tooltip: 'Limpiar',
                            onPressed: () {
                              busqueda.clear();
                              setState(() => filtro = '');
                            },
                            icon: const Icon(Icons.close),
                          ),
                  ),
                ),
              ),
              const SizedBox(width: 12),
              if (esMedio(context))
                Chip(
                  avatar: const Icon(Icons.inventory_2_outlined, size: 16),
                  label: Text(
                    '${productos.length} producto${productos.length == 1 ? '' : 's'}',
                  ),
                ),
            ],
          ),
        ),
        Expanded(
          child: productos.isEmpty
              ? EmptyState(
                  icon: controller.productos.isEmpty
                      ? Icons.inventory_2_outlined
                      : Icons.search_off,
                  title: controller.productos.isEmpty
                      ? 'Sin productos en el catálogo'
                      : 'Sin resultados',
                  message: controller.productos.isEmpty
                      ? 'Actualiza el catálogo cuando tengas conexión para vender desde este dispositivo.'
                      : 'Prueba con otro nombre o categoría.',
                  action: controller.productos.isEmpty
                      ? FilledButton.icon(
                          onPressed: () async {
                            try {
                              await controller.actualizarCatalogo();
                              if (context.mounted) {
                                ScaffoldMessenger.of(context).showSnackBar(
                                  const SnackBar(
                                    content: Text('Catálogo actualizado'),
                                  ),
                                );
                              }
                            } catch (e) {
                              if (context.mounted) {
                                ScaffoldMessenger.of(context).showSnackBar(
                                  SnackBar(content: Text(e.toString())),
                                );
                              }
                            }
                          },
                          icon: const Icon(Icons.sync),
                          label: const Text('Actualizar catálogo'),
                        )
                      : null,
                )
              : LayoutBuilder(
                  builder: (context, constraints) {
                    final columnas = constraints.maxWidth >= 1180
                        ? 3
                        : constraints.maxWidth >= 700
                        ? 2
                        : 1;
                    return GridView.builder(
                      padding: const EdgeInsets.fromLTRB(20, 0, 20, 20),
                      gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                        crossAxisCount: columnas,
                        crossAxisSpacing: 12,
                        mainAxisSpacing: 12,
                        mainAxisExtent: 152,
                      ),
                      itemCount: productos.length,
                      itemBuilder: (context, index) {
                        final p = productos[index];
                        return _ProductCard(
                          producto: p,
                          existencia: controller.existenciasMil[p.id],
                          enCarrito: carrito[p.id] ?? 0,
                          onAdd: () => _agregar(p),
                          onTap: () => cantidad(p),
                        );
                      },
                    );
                  },
                ),
        ),
      ],
    );

    final panel = _CartPanel(
      carrito: carrito,
      porId: porId,
      total: total,
      guardando: guardando,
      metodoPago: metodoPago,
      onMetodo: (metodo) => setState(() => metodoPago = metodo),
      onDelta: _agregar,
      onRemove: (id) => setState(() => carrito.remove(id)),
      onClear: () => setState(carrito.clear),
      onSave: confirmar,
    );

    if (esAncho(context)) {
      return Padding(
        padding: const EdgeInsets.only(top: 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Expanded(child: catalogo),
            Padding(
              padding: const EdgeInsets.fromLTRB(0, 0, 20, 20),
              child: SizedBox(width: 360, child: panel),
            ),
          ],
        ),
      );
    }

    return Column(
      children: [
        Expanded(child: catalogo),
        if (carrito.isNotEmpty)
          SafeArea(
            top: false,
            child: Padding(
              padding: const EdgeInsets.fromLTRB(20, 0, 20, 12),
              child: SizedBox(
                height: (280 + carrito.length * 55).clamp(360, 430).toDouble(),
                child: panel,
              ),
            ),
          ),
      ],
    );
  }
}

class _ProductCard extends StatelessWidget {
  const _ProductCard({
    required this.producto,
    required this.existencia,
    required this.enCarrito,
    required this.onAdd,
    required this.onTap,
  });

  final Producto producto;
  final int? existencia;
  final int enCarrito;
  final VoidCallback onAdd;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Card(
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 10, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: Text(
                      producto.nombre,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  if (enCarrito > 0) ...[
                    const SizedBox(width: 6),
                    Badge(label: Text(scaledText(enCarrito, 3))),
                  ],
                ],
              ),
              const SizedBox(height: 2),
              Text(
                '${producto.categoria} · ${producto.unidad}',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: scheme.onSurfaceVariant,
                ),
              ),
              const Spacer(),
              Row(
                children: [
                  Text(
                    dinero(
                      subtotalCentavos(1000, precioDiezmiles(producto.precio)),
                    ),
                    style: theme.textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                      color: scheme.primary,
                    ),
                  ),
                  const SizedBox(width: 8),
                  _StockChip(existencia: existencia),
                  const Spacer(),
                  IconButton.filledTonal(
                    tooltip: 'Agregar 1 ${producto.unidad}',
                    onPressed: onAdd,
                    icon: const Icon(Icons.add, size: 20),
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _StockChip extends StatelessWidget {
  const _StockChip({required this.existencia});

  final int? existencia;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final String label;
    final Color bg;
    final Color fg;
    if (existencia == null) {
      label = 'Sin dato';
      bg = scheme.surfaceContainerHighest;
      fg = scheme.onSurfaceVariant;
    } else if (existencia! < 0) {
      label = 'Negativo';
      bg = scheme.errorContainer;
      fg = scheme.onErrorContainer;
    } else {
      label = scaledText(existencia!, 3);
      bg = scheme.secondaryContainer;
      fg = scheme.onSecondaryContainer;
    }
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: fg,
        ),
      ),
    );
  }
}

class _CartPanel extends StatelessWidget {
  const _CartPanel({
    required this.carrito,
    required this.porId,
    required this.total,
    required this.guardando,
    required this.metodoPago,
    required this.onMetodo,
    required this.onDelta,
    required this.onRemove,
    required this.onClear,
    required this.onSave,
  });

  final Map<String, int> carrito;
  final Map<String, Producto> porId;
  final int total;
  final bool guardando;
  final String metodoPago;
  final ValueChanged<String> onMetodo;
  final void Function(Producto producto, int delta) onDelta;
  final ValueChanged<String> onRemove;
  final VoidCallback onClear;
  final VoidCallback onSave;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Icon(Icons.shopping_cart_checkout, color: scheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Venta actual',
                    style: theme.textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                if (carrito.isNotEmpty)
                  IconButton(
                    tooltip: 'Vaciar carrito',
                    onPressed: onClear,
                    icon: const Icon(Icons.delete_sweep_outlined),
                  ),
              ],
            ),
            const SizedBox(height: 4),
            Expanded(
              child: carrito.isEmpty
                  ? const EmptyState(
                      icon: Icons.point_of_sale,
                      title: 'Carrito vacío',
                      message:
                          'Toca un producto o usa el botón + para agregarlo.',
                    )
                  : ListView.separated(
                      itemCount: carrito.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        final entry = carrito.entries.elementAt(index);
                        final producto = porId[entry.key];
                        if (producto == null) return const SizedBox.shrink();
                        return _CartLine(
                          producto: producto,
                          cantidad: entry.value,
                          onDelta: (delta) => onDelta(producto, delta),
                          onRemove: () => onRemove(entry.key),
                        );
                      },
                    ),
            ),
            const Divider(height: 20),
            Text('Método de pago', style: theme.textTheme.labelLarge),
            const SizedBox(height: 6),
            SegmentedButton<String>(
              segments: const [
                ButtonSegment(
                  value: 'efectivo',
                  icon: Icon(Icons.payments_outlined, size: 18),
                  label: Text('Efectivo'),
                ),
                ButtonSegment(
                  value: 'tarjeta',
                  icon: Icon(Icons.credit_card, size: 18),
                  label: Text('Tarjeta'),
                ),
              ],
              selected: {metodoPago},
              showSelectedIcon: false,
              onSelectionChanged: (values) => onMetodo(values.first),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Text('Total', style: theme.textTheme.titleMedium),
                const Spacer(),
                Text(
                  dinero(total),
                  style: theme.textTheme.headlineSmall?.copyWith(
                    fontWeight: FontWeight.w800,
                    color: scheme.primary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            FilledButton.icon(
              onPressed: carrito.isEmpty || guardando ? null : onSave,
              icon: guardando
                  ? const SizedBox(
                      height: 18,
                      width: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : Icon(
                      metodoPago == 'tarjeta'
                          ? Icons.credit_card
                          : Icons.check,
                    ),
              label: Text(
                guardando
                    ? 'Guardando…'
                    : metodoPago == 'tarjeta'
                    ? 'Cobrar con tarjeta'
                    : 'Guardar venta',
              ),
            ),
            const SizedBox(height: 8),
            Text(
              metodoPago == 'tarjeta'
                  ? 'Completa el cobro con la tarjeta.'
                  : 'Se guarda en el dispositivo aunque no haya conexión.',
              textAlign: TextAlign.center,
              style: theme.textTheme.bodySmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _CartLine extends StatelessWidget {
  const _CartLine({
    required this.producto,
    required this.cantidad,
    required this.onDelta,
    required this.onRemove,
  });

  final Producto producto;
  final int cantidad;
  final ValueChanged<int> onDelta;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          IconButton.filledTonal(
            tooltip: 'Quitar una unidad',
            visualDensity: VisualDensity.compact,
            onPressed: () => onDelta(-1000),
            icon: const Icon(Icons.remove, size: 18),
          ),
          SizedBox(
            width: 52,
            child: Text(
              scaledText(cantidad, 3),
              textAlign: TextAlign.center,
              style: const TextStyle(fontWeight: FontWeight.w600),
            ),
          ),
          IconButton.filledTonal(
            tooltip: 'Agregar una unidad',
            visualDensity: VisualDensity.compact,
            onPressed: () => onDelta(1000),
            icon: const Icon(Icons.add, size: 18),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  producto.nombre,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                Text(
                  '${dinero(subtotalCentavos(cantidad, precioDiezmiles(producto.precio)))} · ${producto.unidad}',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          IconButton(
            tooltip: 'Quitar del carrito',
            onPressed: onRemove,
            icon: const Icon(Icons.close, size: 18),
          ),
        ],
      ),
    );
  }
}

class HistorialPage extends StatelessWidget {
  const HistorialPage({super.key, required this.controller});
  final PosController controller;

  @override
  Widget build(BuildContext context) {
    final ventas = controller.ventas;
    if (ventas.isEmpty) {
      return const Center(
        child: Text('Todavía no hay ventas en este dispositivo.'),
      );
    }
    return ListView.builder(
      itemCount: ventas.length,
      itemBuilder: (context, index) {
        final venta = ventas[index];
        final color = switch (venta.estado) {
          'aceptada' => Colors.green,
          'aceptada_con_incidencia' => Colors.orange,
          'rechazada' => Colors.red,
          _ => Colors.blueGrey,
        };
        final lines = venta.cuerpo['renglones'] as List<dynamic>;
        return Card(
          margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 5),
          child: ListTile(
            leading: Icon(Icons.circle, color: color, size: 16),
            title: Text(
              '${lines.length} producto(s) · ${venta.estado.replaceAll('_', ' ')}',
            ),
            subtitle: Text(
              '${venta.fecha}\n${venta.mensaje ?? venta.id}',
              maxLines: 3,
              overflow: TextOverflow.ellipsis,
            ),
            isThreeLine: true,
            trailing: venta.estado == 'rechazada'
                ? IconButton(
                    tooltip: 'Reintentar sin modificar la venta',
                    icon: const Icon(Icons.replay),
                    onPressed: () async {
                      try {
                        await controller.reintentar(venta);
                      } catch (e) {
                        if (context.mounted) {
                          ScaffoldMessenger.of(
                            context,
                          ).showSnackBar(SnackBar(content: Text(e.toString())));
                        }
                      }
                    },
                  )
                : null,
          ),
        );
      },
    );
  }
}

class CatalogoPage extends StatelessWidget {
  const CatalogoPage({super.key, required this.controller});
  final PosController controller;

  @override
  Widget build(BuildContext context) => ListView(
    padding: const EdgeInsets.all(12),
    children: [
      Text(
        'Catálogo de ${controller.usuario!.puestoId}',
        style: Theme.of(context).textTheme.titleLarge,
      ),
      const SizedBox(height: 8),
      const Text(
        'Los precios y existencias se actualizaron por última vez al conectar con la API.',
      ),
      const SizedBox(height: 12),
      for (final p in controller.productos)
        Card(
          child: ListTile(
            title: Text(p.nombre),
            subtitle: Text('${p.categoria} · ${p.unidad}'),
            trailing: Text(
              dinero(subtotalCentavos(1000, precioDiezmiles(p.precio))),
            ),
          ),
        ),
    ],
  );
}
