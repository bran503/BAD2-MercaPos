# MercaPOS Backend

> **Estado 2026-11-01:** compila, pasa Clippy, las pruebas unitarias y tres pruebas de integración con CouchDB aislado (`auth_sync_integration_test`, `producto_repository_test` y `paginacion_couchdb_test`). `/health` respondió HTTP 200 con CouchDB activo. Incluye autenticación, permisos por puesto, sincronización por lotes e incidencias. Consulte [RESULTADOS_PRUEBAS.md](RESULTADOS_PRUEBAS.md) y [CONTRATO_FRONTEND.md](CONTRATO_FRONTEND.md).

Backend HTTP para MercaPOS, desarrollado en Rust con Axum y Apache CouchDB. La API aplica validaciones de productos, usuarios, puestos, operaciones de venta e inventario; guarda documentos JSON en CouchDB y calcula las existencias a partir de los movimientos registrados.

## Requisitos

- Rust estable y Cargo.
- Apache CouchDB disponible localmente.
- Las bases CouchDB `mercapos` y `mercapos_auth` creadas y accesibles con las credenciales configuradas. Las credenciales y sesiones viven en `mercapos_auth`, separadas de los datos de negocio.

## Iniciar CouchDB

En el proyecto de base de datos MercaPOS, inicia el contenedor:

```bash
docker compose up -d
```

Si es la primera ejecución, crea las bases `mercapos` y `mercapos_auth`, y configura los índices Mango de negocio usando las solicitudes de configuración de ese proyecto. La guía de base de datos describe la secuencia completa. CouchDB debe ser accesible desde el backend, por ejemplo en `http://127.0.0.1:5985/`.

## Configuración

Crea un archivo `.env` en la raíz del backend con tus datos locales:

```env
COUCHDB_URL=http://127.0.0.1:5985/
COUCHDB_USERNAME=mercapos_api
COUCHDB_PASSWORD=tu-clave-de-servicio
COUCHDB_DATABASE=mercapos
COUCHDB_AUTH_DATABASE=mercapos_auth
API_ADDRESS=127.0.0.1:3000
MERCAPOS_BOOTSTRAP_TOKEN=cambie-este-token-de-un-solo-uso
MERCAPOS_SESSION_TTL_SECONDS=28800
MERCAPOS_CORS_ORIGINS=http://localhost:5173,http://127.0.0.1:5173
```

`MERCAPOS_BOOTSTRAP_TOKEN` habilita `POST /auth/bootstrap` mientras no exista ninguna credencial; quítelo después del alta inicial. El bootstrap se serializa dentro de la instancia escritora. `MERCAPOS_SESSION_TTL_SECONDS` y `MERCAPOS_CORS_ORIGINS` son opcionales. Credenciales y sesiones se guardan en la base separada `mercapos_auth`, que no debe replicarse; las contraseñas se guardan con Argon2id. Login bloquea una cuenta por 15 minutos tras cinco fallos en la ventana.

No guardes credenciales reales en Git. Crea primero la cuenta de servicio `mercapos_api` siguiendo `mercapos-db/MIGRACION.md`; no configures el backend con la cuenta admin. `API_ADDRESS` es opcional; si no está definido, la API usa `127.0.0.1:3000`. Para la demo local, deja esa dirección para que el servicio escuche solo en el equipo.

## Ejecutar la API

Desde la carpeta del backend:

```bash
cargo run
```

La API queda disponible en `http://127.0.0.1:3000`.

Comprueba el estado de la API y CouchDB:

```bash
curl -i http://127.0.0.1:3000/health
```

Una respuesta HTTP `200` con `api: "ok"` confirma que el backend pudo consultar CouchDB. Las métricas están en:

```bash
curl -i -H "Authorization: Bearer $TOKEN_ADMIN" http://127.0.0.1:3000/metrics
```

La respuesta de `/metrics` usa formato Prometheus y requiere una sesión válida de administrador. Cada solicitud también genera un registro estructurado e incluye un `X-Request-Id` en la respuesta.

## Pruebas

El comando estándar no accede a una base real; la integración CouchDB se marcó como prueba ignorada y exige una base `mercapos_test_*` explícita. Ejecute:

```bash
cargo test
```

Las pruebas de integración se ejecutan por separado con `--ignored`. Configure `MERCAPOS_TEST_URL`, `MERCAPOS_TEST_USERNAME`, `MERCAPOS_TEST_PASSWORD`, `MERCAPOS_TEST_DATABASE` y `MERCAPOS_TEST_AUTH_DATABASE` con bases aisladas: `mercapos_test_*` y `mercapos_test_auth_*` respectivamente.

La base `MERCAPOS_TEST_DATABASE` debe tener instalado el esquema v2 (diseños e índices) con `python3 mercapos-db/tools/instalar_esquema.py --database <base>`; la prueba de autenticación y sincronización falla de forma explícita si falta el diseño `validacion_v2`, porque la API debe escribir documentos que el validador de CouchDB acepte. Como el esquema v2 no permite borrar documentos comerciales, la prueba `paginacion_couchdb_test` usa 25 productos con identificadores estables (`producto:paginacion_0` … `producto:paginacion_24`) que se sobrescriben en cada corrida, consulta páginas de 10 y deja esos fixtures en la base de ensayo.

```bash
cargo test --test auth_sync_integration_test --test producto_repository_test -- --ignored
cargo test --test paginacion_couchdb_test -- --ignored
```

`auth_sync_integration_test` cubre login, sesión, permisos por rol/puesto, venta autenticada, lote offline con estados e incidencias, resolución y logout. Las pruebas de lógica de negocio no dependen de CouchDB:

```bash
cargo test --test operacion_use_cases_test --test producto_use_cases_test --test usuario_use_cases_test
```

Para revisar el estilo del código:

```bash
cargo fmt --check
```

## Endpoints

Todas las rutas usan JSON salvo `/metrics`. Las respuestas de negocio y persistencia con error tienen la forma `{"error":"mensaje"}`. Los IDs se envían como `id`; al guardarlos en CouchDB se convierten en `_id`. Usa prefijos como `puesto:`, `usuario:`, `producto:`, `operacion:`, `mov:` y `conteo:`.

| Método | Ruta | Uso |
| --- | --- | --- |
| `GET` | `/health` | Estado del backend y CouchDB |
| `GET` | `/metrics` | Métricas Prometheus |
| `GET` | `/puestos` | Lista el puesto autenticado; acepta `?activo=true` |
| `GET`, `PUT` | `/puestos/{id}` | Consultar y actualizar un puesto |
| `GET` | `/puestos/{id}/usuarios` | Listar usuarios del puesto |
| `GET` | `/puestos/{id}/existencias` | Consultar existencias calculadas desde los movimientos |
| `POST` | `/usuarios` | Crear usuario |
| `GET`, `PUT` | `/usuarios/{id}` | Consultar y actualizar un usuario |
| `POST`, `GET` | `/productos` | Crear productos y listarlos; `GET` exige `puesto_id` y acepta `activo` |
| `GET`, `PUT` | `/productos/{id}` | Consultar y actualizar un producto |
| `POST`, `GET` | `/operaciones` | Registrar una operación o listarlas con `?puesto_id=...` |
| `GET` | `/operaciones/{id}` | Consultar una operación |
| `POST`, `GET` | `/conteos` | Registrar un conteo o listarlos con `?puesto_id=...` |
| `GET` | `/conteos/{id}` | Consultar un conteo |
| `POST`, `GET` | `/movimientos` | Crear inventario inicial o listar por `puesto_id` / `producto_id` |

### Flujo local recomendado

1. Ejecutar el bootstrap único (`POST /auth/bootstrap`) para crear el primer puesto y administrador. Después el administrador puede crear usuarios de su puesto y productos activos.
2. Inicializar el inventario mediante `POST /movimientos`, usando `motivo: "inventario_inicial"`, `usuario_id` y una cantidad positiva. La apertura se registra una vez por producto y puesto.
3. Registrar ventas, compras, mermas, devoluciones y ajustes mediante `POST /operaciones`. Una operación nueva guarda todos sus efectos en un solo documento; `/movimientos` los proyecta.
4. Registrar conteos con período, etapa, origen y evidencia según corresponda. Un ajuste con `conteo_id` debe coincidir con el conteo POS y solo se aplica una vez.
5. Consultar movimientos y existencias para confirmar los saldos.

Las ventas y mermas reducen existencias; compras y devoluciones vendibles de clientes las aumentan. Una devolución `no_vendible` no aumenta stock. Un ajuste usa la diferencia con signo. La API exige IDs de operación estables: reenviar el mismo ID y contenido completo responde `200` sin crear efectos adicionales; reutilizar un ID con datos diferentes responde `409`. Un documento legacy incompleto se informa como conflicto para revisión, sin crear salidas nuevas.

La guía detallada de solicitudes, respuestas y validaciones está en [ENDPOINTS.md](ENDPOINTS.md).

## Alcance de la demo local

La API autentica clientes con sesiones Bearer y limita usuarios y administradores comerciales a su puesto. La coordinación de inventario funciona dentro de una instancia escritora; al desplegar, mantenga esa topología. El navegador envía operaciones a la API y no replica directamente los documentos de CouchDB. El laboratorio de replicación bidireccional de los archivos HTTP se conserva por separado.
