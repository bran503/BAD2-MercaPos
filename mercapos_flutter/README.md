# MercaPOS Flutter

Cliente Flutter web para la API Rust de este directorio de trabajo. Incluye ventas, catálogo, inventario, operaciones, conteos, usuarios, incidencias y configuración del puesto. Android queda pospuesto hasta que el usuario lo solicite.

## Arquitectura

- En la web, `sembast_web` guarda productos y el cuerpo exacto de cada operación en IndexedDB. Las ventas se confirman localmente **antes** de intentar enviarlas.
- `flutter_secure_storage` guarda el token, identidad, dirección de API e identificador del dispositivo. Su implementación web requiere HTTPS o `localhost`.
- `http` envía la cola a `POST /sync/operaciones`, hasta 100 operaciones por lote. El ID UUID y el JSON original permanecen iguales al reintentar.
- `aceptada` y `aceptada_con_incidencia` quedan en el historial local. `rechazada` también permanece visible; se puede reintentar manualmente si se resolvió la causa. Un timeout o una respuesta incompleta deja el lote pendiente.
- CouchDB solo lo usa la API Rust. El navegador y el celular no reciben credenciales de CouchDB ni replican documentos directamente.

El catálogo guardado permite registrar ventas sin red después de un primer inicio de sesión. Si la API devuelve 401, la web elimina la sesión vencida y vuelve al acceso; la cola local se conserva y se sincroniza tras entrar de nuevo. Cada usuario envía solo las operaciones que guardó en su puesto. Cerrar sesión no borra operaciones pendientes. En navegador, los datos locales pertenecen a ese perfil y origen; borrar los datos del sitio o usar modo incógnito puede borrar la cola.

El saldo mostrado es el último saldo central recibido. No se descuenta localmente y puede estar desactualizado, sobre todo con varios dispositivos. Las ventas físicas offline se concilian en el servidor; una aceptación con incidencia puede representar stock central insuficiente o precio distinto. No se asume que el saldo local sea definitivo.

## Ejecutar la web

Flutter 3.47.5 y Dart 3.13.4 están instalados en `~/develop/flutter` y disponibles en nuevas terminales. Para levantar todo desde cero, usa tres terminales:

```bash
cd /home/duran/Escritorio/BD2/mercapos-db
docker compose up -d
```

```bash
cd /home/duran/Escritorio/BD2/mercapos_api
cargo run
```

```bash
cd /home/duran/Escritorio/BD2/mercapos_flutter
flutter run -d chrome --web-hostname localhost --web-port 5173
```

Abre `http://localhost:5173` si Chrome no se abre solo. Para servir la compilación estática en lugar del modo de desarrollo, ejecuta `flutter build web` y luego `python3 -m http.server 5173 --bind 127.0.0.1 --directory build/web`. Solo uno de esos servidores puede ocupar el puerto 5173 a la vez.

Para comprobar cambios: `flutter analyze`, `flutter test --platform chrome` y `flutter build web --no-wasm-dry-run`.

La API local usa `http://127.0.0.1:3000`. El backend permite por defecto los orígenes `http://localhost:5173` y `http://127.0.0.1:5173`; si cambias puerto u origen, ajusta `MERCAPOS_CORS_ORIGINS` antes de arrancarlo. La web debe abrirse en `localhost` para que funcione el almacenamiento seguro. Para publicar, usa HTTPS tanto en la web como en la API.

La app intenta actualizar catálogo y sincronizar al iniciar sesión, al abrirla y después de guardar una venta. También ofrece botones manuales para ambas acciones.

## Alcance web actual

La interfaz permite alta y edición de productos, apertura de inventario, consulta de existencias y movimientos, compras, mermas, devoluciones, conteos y ajustes vinculados, administración de usuarios, resolución de incidencias, edición del puesto y cambio de contraseña. Las ventas funcionan sin conexión con catálogo local. Las otras operaciones se guardan en la cola local y se envían al recuperar conexión; productos, inventario inicial, conteos, usuarios, puesto e incidencias requieren conexión.
