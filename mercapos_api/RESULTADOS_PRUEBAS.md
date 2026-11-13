# Resultados de comprobación

Fecha: 2026-11-01. Entorno: Linux, Rust/Cargo 1.99.1, Python 3.12.3, Node 22.16.0, Docker CLI 29.7.2.

## Verificación actual con CouchDB

El 2026-11-01 se comprobó que el contenedor `mercapos-couchdb` estaba saludable y que `GET /health` de la API local devolvía HTTP 200 con `couchdb.status: ok`. Pasaron `cargo test` (30 pruebas locales), `cargo clippy --all-targets -- -D warnings` y las tres pruebas ignoradas ejecutadas explícitamente contra `mercapos_test_crud` y `mercapos_test_auth_crud`: autenticación/sincronización, CRUD de producto y paginación de 25 productos. Estas pruebas escriben solo en las bases de ensayo y usan IDs de prueba. No se ejecutó migración de la base operativa ni restauración de un respaldo.

La tabla siguiente conserva los resultados y límites registrados antes de esta verificación; las filas que dicen “no se repitió” ya quedaron superadas para las tres pruebas de integración mencionadas arriba.

| Comprobación | Resultado |
| --- | --- |
| `cargo fmt --check` | Pasó en esta sesión. |
| `cargo check --all-targets --locked` | Pasó en esta sesión. |
| `cargo test --locked` | Pasó: 30 pruebas unitarias/de casos de uso; 3 pruebas que requieren CouchDB están ignoradas por defecto. |
| Limitador de intentos login | Prueba unitaria nueva: cinco fallos bloquean la cuenta; un login exitoso limpia el contador. |
| Etiquetas de telemetría | Prueba HTTP local confirma que Prometheus etiqueta `/items/{id}` y no incluye el ID concreto del documento. |
| `cargo clippy --all-targets --locked -- -D warnings` | Pasó en esta sesión. |
| Telemetría | `/metrics` expone contadores y duración agregada por método, patrón de ruta Axum y estado; no incluye IDs como etiquetas. Ahora requiere sesión válida de administrador; una prueba confirma que vendedor recibe 403. |
| Snapshot e instante del servidor | Operaciones nuevas guardan nombre y unidad del catálogo por renglón, más `recibido_en_epoch_ms`; una prueba confirma que los snapshots y la recepción no alteran la comparación de idempotencia. |
| `python3 -m unittest discover -s mercapos-db/tools -p 'test_*.py' -v` | 7 pruebas pasaron: saldos 63/29, anomalía legacy, duplicación semántica, cantidades, reintento simulado, resolución de dos hojas (incluida la elegida) y simulación sin escrituras. |
| Paginación CouchDB con 25 documentos | Según el usuario, pasó desde otra sesión de Codex contra `mercapos_test_crud`: creó 25 productos con IDs únicos, recorrió páginas de 10 y eliminó sus fixtures sin errores. |
| Resolver conflictos | Se añadió herramienta en modo simulación por defecto. Pruebas locales cubren resolución de dos hojas, incluida la elegida, y que la simulación no escriba. No se aplicó a una base real. |
| Mapa/validador CouchDB ejecutados en Node sobre fixture | Simulación local: 16 documentos aceptados, tres inválidos rechazados, vista `existencias_mil` dio 63000/29000. No es una ejecución en CouchDB. |
| `producto_repository_test --ignored` | El usuario reporta que pasó contra CouchDB 3.3.3 en `mercapos_test_crud`. No se repitió desde esta sesión. |
| `auth_sync_integration_test --ignored` | El usuario reporta que pasó contra `mercapos_test_crud` y `mercapos_test_auth_crud`, con bases separadas, incluyendo la venta offline sin stock y la existencia de `-94000` milésimas. No se repitió desde esta sesión. |
| Acceso local TCP `127.0.0.1:5985` | En otra sesión de Codex, `GET /_up` devolvió HTTP 200 y la cuenta `mercapos_api` se autenticó con HTTP 200. Esta sesión no tuvo acceso al loopback local. |

Según el usuario, se crearon las bases `mercapos_auth`, `mercapos_test_auth_crud` y se habilitó el rol `mercapos_writer` para la cuenta de servicio en negocio y autenticación. La API usa `COUCHDB_AUTH_DATABASE` (por defecto `mercapos_auth`) para credenciales y sesiones, separada de `COUCHDB_DATABASE`. No se limpió ningún volumen ni hubo push/despliegue desde esta sesión.

La venta física offline con stock central insuficiente se conserva como `aceptada_con_incidencia`, deja existencia negativa y crea una incidencia durable; su conciliación es una acción administrativa explícita, no un ajuste automático del historial. Rate limiting y serialización del bootstrap están implementados y tienen cobertura local; la serialización solo coordina una instancia escritora. El usuario definió que unos 25 registros son suficientes para el proyecto universitario; el repositorio recorre páginas internamente y no ofrece cursor público porque los listados se entregan completos. La medición de recursos y las pruebas integrales con CouchDB no se repitieron desde esta sesión.

## Verificación end-to-end con el esquema v2 instalado (2026-11-01, tarde)

Se encontró un defecto real: con el diseño `validacion_v2` instalado (el que produce `mercapos-db/tools/instalar_esquema.py`), la API escribía campos opcionales como `null` en los documentos de operación y CouchDB los rechazaba con `403 forbidden: Renglón de operación inválido`. La respuesta de la API era `500 Error interno al procesar la solicitud`, por lo que **ninguna venta, compra, merma, devolución ni ajuste se guardaba** en la base operativa `mercapos` ni en una base con esquema instalado. Las pruebas de integración no lo detectaban porque `mercapos_test_crud` no tenía los diseños instalados.

Correcciones aplicadas:

- La API omite los campos opcionales sin valor en los documentos de operación y conteo, en lugar de escribirlos como `null` (`src/infrastructure/couchdb/repositories/operacion_repository.rs`, `.../conteo_repository.rs`).
- El cliente de CouchDB registra el estado y el motivo de cada rechazo para que un `403` del validador no vuelva a ser invisible (`src/infrastructure/couchdb/client.rs`).
- `mercapos_test_crud` tiene instalado el esquema v2 y la prueba de autenticación/sincronización exige el diseño `validacion_v2`, de modo que el defecto no pueda reaparecer sin fallar.
- `paginacion_couchdb_test` ya no intenta borrar fixtures: el esquema v2 prohíbe borrar documentos comerciales, así que usa identificadores estables y los sobrescribe.

Con el esquema instalado, en bases efímeras `mercapos_smoke` y `mercapos_smoke_auth` (creadas, con rol `mercapos_writer` y esquema v2, y eliminadas al terminar) se ejecutó un flujo completo contra la API real: bootstrap 201 y un solo uso (409 en el segundo intento), login correcto e incorrecto, `GET /auth/me`, creación de producto, movimiento de inventario inicial, venta en línea 201, venta en línea sin stock 422 con `codigo: stock_insuficiente`, venta offline sin stock por `/sync/operaciones` como `aceptada_con_incidencia` con existencia `-96000` milésimas, incidencia listada, conteo POS 201, ajuste que coincide con el conteo 201, segundo ajuste del mismo conteo 409 y logout 204 con token invalidado. Todas las comprobaciones pasaron.

Además, en la misma sesión: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` y `cargo test --locked` (30 pruebas) pasan; las tres integraciones contra `mercapos_test_crud` y `mercapos_test_auth_crud` pasan; `python3 -m unittest discover -s mercapos-db/tools -p 'test_*.py' -v` pasa 9 pruebas; y en el frontend `flutter analyze` no reporta problemas, `flutter test --platform chrome` pasa 6 pruebas y `flutter build web --release` compila. La API local quedó reiniciada en `127.0.0.1:3000` con el binario corregido.

## Pago con tarjeta simulado (2026-11-11)

Se añadió el campo opcional `pago` a las ventas, validado en la API y en `mercapos-db/design_docs/validacion_v2.js` (instalado en `mercapos` y `mercapos_test_crud`). La API omite `pago` cuando no existe y lo persiste y devuelve cuando está presente. La prueba de integración de autenticación/sincronización crea una venta con pago aprobado y verifica que `GET /operaciones/{id}` devuelve la autorización. En el frontend, el diálogo de POS ficticio permite elegir aprobado/rechazado/cancelado, el comprobante se muestra tras aprobar y la impresión web usa un Blob; rechazado o cancelado conserva el carrito sin crear venta.

Verificación: `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked` (32 pruebas) y las tres integraciones con CouchDB aislado pasan. Frontend: `flutter analyze` limpio, `flutter test --platform chrome` pasa 8 pruebas (incluye dos del POS simulado) y `flutter build web --release` compila.
