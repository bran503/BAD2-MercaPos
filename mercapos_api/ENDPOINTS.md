# API MercaPOS

> **Contrato 2026-11-01.** El backend compila, pasa Clippy y las pruebas unitarias. La integración auth/sync no se ha vuelto a correr tras separar la base de autenticación; el usuario reportó una ejecución satisfactoria del CRUD de producto contra CouchDB 3.3.3.

Esta API en Rust aplica las reglas de ventas e inventario y persiste documentos JSON en CouchDB. Una operación nueva guarda sus impactos de inventario dentro de un solo documento `schema_version: 2`; `GET /movimientos` proyecta esos efectos. Los documentos legacy se leen para migración y consulta.

## Inicio

Configura estas variables de entorno (también se leen desde `.env`) y ejecuta `cargo run`:

```env
COUCHDB_URL=http://127.0.0.1:5985/
COUCHDB_USERNAME=mercapos_api
COUCHDB_PASSWORD=clave-de-servicio-local
COUCHDB_DATABASE=mercapos
COUCHDB_AUTH_DATABASE=mercapos_auth
API_ADDRESS=127.0.0.1:3000
MERCAPOS_BOOTSTRAP_TOKEN=cambie-este-token-de-un-solo-uso
MERCAPOS_SESSION_TTL_SECONDS=28800
MERCAPOS_CORS_ORIGINS=http://localhost:5173,http://127.0.0.1:5173
```

La dirección puede ser `0.0.0.0:3000` para permitir que otro dispositivo de la red alcance la API. No expongas CouchDB directamente a internet. La base debe estar creada, con los índices Mango y la vista de existencias descritos en el proyecto `mercapos-db`.

Todas las rutas salvo `/`, `/health`, `/auth/login` y `/auth/bootstrap` exigen `Authorization: Bearer <token>`. `/metrics` requiere además el rol administrador del puesto. Configure la base separada de autenticación con `COUCHDB_AUTH_DATABASE`; no la replique ni envíe credenciales de CouchDB al cliente.

Base URL local: `http://127.0.0.1:3000`.

## Formato acordado con CouchDB

Los documentos de negocio comparten `COUCHDB_DATABASE`; las credenciales y sesiones se guardan en `COUCHDB_AUTH_DATABASE`, separada. Todos usan `tipo` como discriminador y `_id` como identificador. La API muestra ese identificador como `id`. Los prefijos recomendados son `puesto:`, `usuario:`, `producto:`, `operacion:`, `mov:` y `conteo:`. Los IDs de operación deben ser generados por el dispositivo antes del envío y conservarse en los reintentos.

| Recurso | `tipo` en CouchDB | Persistencia |
| --- | --- | --- |
| Puesto | `puesto` | Editable, con revisión `_rev` |
| Usuario | `usuario` | Editable, sin contraseñas |
| Producto | `producto` | Editable, con revisión `_rev` |
| Operación | `operacion` | Inmutable, creada con ID determinista |
| Movimiento | `movimiento` | Inmutable; se genera desde la operación o por inventario inicial |
| Conteo | `conteo_fisico` | Inmutable |

Las cantidades enviadas en ventas, compras, mermas y devoluciones son positivas. En ajustes, `cantidad` puede ser positiva o negativa y expresa la diferencia física. El documento `movimiento` guarda esa cantidad con signo: positiva para entrada y negativa para salida. Se añade `tipo_devolucion` (`cliente` o `proveedor`) a la operación de devolución para determinar si el movimiento resultante entra o sale del inventario.

## Endpoints

### Autenticación

| Método y ruta | Uso |
| --- | --- |
| `POST /auth/bootstrap` | Crea el primer puesto, administrador y credencial; exige `X-Bootstrap-Token` y que no exista ninguna credencial |
| `POST /auth/login` | Inicia sesión y devuelve `token`, `expira_en` y `usuario` |
| `POST /auth/logout` | Revoca la sesión actual (204) |
| `GET /auth/me` | Devuelve el usuario autenticado |
| `POST /auth/password` | Cambia la contraseña con `{"actual":"...","nueva":"..."}` |

Ejemplo `POST /auth/login`:

```json
{"usuario_id":"usuario:0002","password":"secreto-123"}
```

Respuesta 200:

```json
{"token":"<token-opaco>","expira_en":1767232800,"usuario":{"id":"usuario:0002","tipo":"usuario","revision":"1-...","nombre":"Ana","rol":"administrador","puesto_id":"puesto:0001","activo":true}}
```

Los usuarios nuevos aceptan `password` opcional en `POST /usuarios` y `PUT /usuarios/{id}` (mínimo 8 caracteres). El documento `usuario` nunca guarda la contraseña; se crea un documento `credencial:{usuario_id}` con Argon2id en la base de autenticación separada. Tanto vendedor como administrador comercial se limitan a su `puesto_id`; no hay rol global.

### Sincronización offline

| Método y ruta | Uso |
| --- | --- |
| `POST /sync/operaciones` | Envía hasta 100 operaciones y devuelve un estado por operación |
| `GET /sync/incidencias?puesto_id=&resuelta=false` | Lista incidencias durables; el vendedor solo ve su puesto |
| `PUT /sync/incidencias/{operacion_id}/resolver` | El administrador marca la incidencia como resuelta |

Estados por operación: `aceptada`, `aceptada_con_incidencia` (la venta se guarda aunque el precio difiera del actual o aunque una venta offline no tenga stock suficiente, dejando existencia negativa) y `rechazada` (validación, conflicto, permiso o stock en operaciones que no son ventas offline; se guarda incidencia). La aceptación con stock negativo aplica solo a `/sync/operaciones`; `POST /operaciones` en línea sigue rechazando una venta sin stock. Si falla guardar la incidencia, el lote devuelve error y no confirma el resultado. Los reintentos sobre el mismo `operacion_id` incrementan `intentos` de la incidencia `incidencia:{operacion_id}`.

Solicitud:

```json
{"operaciones":[{"id":"operacion:abc","tipo_operacion":"venta","puesto_id":"puesto:0001","usuario_id":"usuario:0002","dispositivo_id":"pos-1","fecha_local":"2026-11-01T10:00:00-06:00","renglones":[{"producto_id":"producto:1","cantidad":1,"precio_unitario":1.25}],"total":1.25}]}
```

Respuesta 200:

```json
{"resultados":[{"operacion_id":"operacion:abc","estado":"aceptada","mensaje":"Operación aceptada","incidencia_id":null}],"aceptadas":1,"con_incidencia":0,"rechazadas":0}
```

### Estado y telemetría

| Método y ruta | Uso |
| --- | --- |
| `GET /health` | Consulta CouchDB y devuelve el estado de la API |
| `GET /metrics` | Métricas Prometheus: contadores, histogramas de latencia por ruta, hilos y RSS del proceso |

Ejemplo `GET /health` (200):

```json
{"api":"ok","couchdb":{"status":"ok"}}
```

`/metrics` devuelve texto con formato Prometheus y requiere sesión válida de administrador. Configura el recolector Prometheus con un token de ese administrador y guárdalo en el gestor de secretos del recolector. El backend también registra `request_id`, método, ruta, estado y duración mediante `tracing`; inicia con `RUST_LOG=info` si necesitas ajustar el nivel del suscriptor en un despliegue futuro.

### Puestos

| Método y ruta | Uso |
| --- | --- |
| `GET /puestos?activo=true` | Devuelve el puesto autenticado; `activo` es opcional |
| `GET /puestos/{id}` | Obtiene un puesto |
| `PUT /puestos/{id}` | Actualiza un puesto |
| `GET /puestos/{puesto_id}/usuarios` | Lista usuarios del puesto |
| `GET /puestos/{id}/existencias` | Calcula las existencias sumando movimientos con signo |

`GET` devuelve `revision` para puesto y usuario. Los `PUT` deben enviar esa revisión en el JSON; una revisión obsoleta devuelve 409. Las rutas también requieren Bearer y limitan las ediciones al puesto autenticado.

No existe alta general de puestos mediante la sesión de un administrador comercial. El primer puesto se crea mediante el bootstrap de una sola vez. La creación de puestos adicionales requiere un procedimiento administrativo global que aún no forma parte de la API.

Ejemplo de respuesta:

```json
{"id":"puesto:0001","tipo":"puesto","nombre":"Puesto de verduras","mercado":"Mercado municipal","ubicacion":"Santa Ana, El Salvador","propietario_usuario_id":"usuario:0002","activo":true}
```

### Usuarios

| Método y ruta | Uso |
| --- | --- |
| `POST /usuarios` | Crea usuario sin credenciales |
| `GET /usuarios/{id}` | Obtiene usuario |
| `PUT /usuarios/{id}` | Actualiza usuario |

Solicitud `POST /usuarios`:

```json
{"id":"usuario:0001","nombre":"Vendedor","rol":"vendedor","puesto_id":"puesto:0001","activo":true}
```

`rol` acepta `vendedor` o `administrador`. La respuesta devuelve el documento con `tipo: "usuario"`; no se almacenan contraseñas en el documento.

### Productos

| Método y ruta | Uso |
| --- | --- |
| `POST /productos` | Crea producto (activo por defecto) |
| `GET /productos?puesto_id=puesto:0001&activo=true` | Lista productos del puesto; `activo` es opcional |
| `GET /productos/{id}` | Obtiene producto |
| `PUT /productos/{id}` | Actualiza producto |

`PUT /productos/{id}` exige `puesto_id` y `revision` obtenida del `GET` previo. La unidad de un producto con historial de inventario no puede cambiarse.

Solicitud `POST /productos`:

```json
{"id":"producto:0042","puesto_id":"puesto:0001","nombre":"Tomate (libra)","categoria":"verduras","precio_unitario":1.25,"unidad":"libra","actualizado_en":"2026-10-30T09:00:00-06:00","actualizado_por":"usuario:0002"}
```

Respuesta (201):

```json
{"mensaje":"Producto creado correctamente","producto":{"id":"producto:0042","puesto_id":"puesto:0001","revision":"1-ejemplo","nombre":"Tomate (libra)","categoria":"verduras","precio_unitario":1.25,"unidad":"libra","activo":true,"actualizado_en":"2026-10-30T09:00:00-06:00","actualizado_por":"usuario:0002"}}
```

### Operaciones y movimientos

| Método y ruta | Uso |
| --- | --- |
| `POST /operaciones` | Registra una operación canónica y proyecta sus movimientos |
| `GET /operaciones/{id}` | Obtiene una operación |
| `GET /operaciones?puesto_id=puesto:0001` | Lista operaciones del puesto |
| `POST /movimientos` | Registra únicamente inventario inicial |
| `GET /movimientos?puesto_id=puesto:0001` | Lista movimientos del puesto |
| `GET /movimientos?producto_id=producto:0042` | Lista movimientos de un producto |

Solicitud `POST /operaciones` para una venta:

```json
{
  "id":"operacion:disp03:20261030T090000",
  "tipo_operacion":"venta",
  "puesto_id":"puesto:0001",
  "usuario_id":"usuario:0001",
  "dispositivo_id":"disp03",
  "fecha_local":"2026-10-30T09:00:00-06:00",
  "renglones":[{"producto_id":"producto:0042","cantidad":2,"precio_unitario":1.25}],
  "total":2.50,
  "pago":{
    "metodo":"tarjeta",
    "modo":"simulado",
    "estado":"aprobado",
    "autorizacion":"AUT-123456",
    "fecha":"2026-10-30T09:00:00-06:00"
  }
}
```

`pago` es opcional y solo válido en ventas: `metodo` debe ser `tarjeta`, `modo` debe ser `simulado`, `estado` uno de `aprobado|rechazado|cancelado`, y `autorizacion`/`fecha` no pueden estar vacías. La API no guarda números ni datos de tarjeta. Los pagos rechazados o cancelados no generan venta.

Respuesta de creación (201); un reintento idéntico devuelve 200 sin nuevas escrituras. Una operación legacy con movimientos faltantes devuelve conflicto para auditoría:

```json
{"id":"operacion:disp03:20261030T090000","tipo":"operacion","tipo_operacion":"venta","puesto_id":"puesto:0001","usuario_id":"usuario:0001","dispositivo_id":"disp03","fecha_local":"2026-10-30T09:00:00-06:00","recibido_en_epoch_ms":1793383200000,"renglones":[{"producto_id":"producto:0042","nombre_producto":"Tomate","unidad":"libra","cantidad":2.0,"precio_unitario":1.25,"costo_unitario":null}],"total":2.5,"total_centavos":250,"motivo":null,"operacion_origen_id":null,"conteo_id":null,"tipo_devolucion":null,"destino_devolucion":null}
```

Tipos y reglas de los renglones:

| `tipo_operacion` | Reglas |
| --- | --- |
| `venta` | `total` y `precio_unitario` por renglón obligatorios; cantidad positiva |
| `compra` | `total` y `costo_unitario` por renglón obligatorios; cantidad positiva |
| `merma` | `motivo` obligatorio; cantidad positiva |
| `devolucion` | `operacion_origen_id` y `tipo_devolucion` obligatorios; cliente usa precio y origen venta, proveedor usa costo y origen compra; cantidad positiva y no mayor a la pendiente |
| `ajuste` | `motivo` obligatorio; cantidad no cero con signo según la diferencia |

Solicitud de ajuste de inventario de una unidad menos:

```json
{"id":"operacion:disp03:20261030T180500","tipo_operacion":"ajuste","puesto_id":"puesto:0001","usuario_id":"usuario:0002","dispositivo_id":"disp03","fecha_local":"2026-10-30T18:05:00-06:00","renglones":[{"producto_id":"producto:0042","cantidad":-1}],"motivo":"diferencia detectada en conteo físico","conteo_id":"conteo:disp03:20261030T180000"}
```

El backend proyecta IDs estables de movimiento como `mov:disp03:20261030T180500:r1:0042`. El cliente no crea movimientos de una operación. El endpoint directo acepta solo `inventario_inicial`, con `usuario_id` de administrador del puesto y cantidad positiva; una segunda apertura del mismo producto y puesto devuelve 409.

### Conteos físicos

| Método y ruta | Uso |
| --- | --- |
| `POST /conteos` | Registra un conteo físico inmutable |
| `GET /conteos/{id}` | Obtiene un conteo |
| `GET /conteos?puesto_id=puesto:0001` | Lista conteos del puesto |

Solicitud `POST /conteos`:

```json
{"id":"conteo:disp03:20261030T180000","puesto_id":"puesto:0001","usuario_id":"usuario:0002","dispositivo_id":"disp03","fecha_local":"2026-10-30T18:00:00-06:00","periodo_estudio":"piloto-01","etapa":"despues","origen_registro":"pos","renglones":[{"producto_id":"producto:0042","cantidad_contada":63}]}
```

En un conteo `pos`, el servidor calcula `cantidad_sistema` bajo el bloqueo de escrituras y guarda `corte_seq`; si el cliente envía una cantidad distinta, devuelve 422. Para `origen_registro: "cuaderno"`, se exige `cantidad_sistema` manual y `evidencia`. La diferencia queda guardada antes del ajuste. Un ajuste vinculado debe coincidir con las diferencias y no puede aplicarse dos veces.

## Errores

Los errores de negocio y de persistencia generados por los handlers usan el formato `{ "error": "mensaje" }`. Una venta en línea sin stock añade `"codigo": "stock_insuficiente"`. Los errores de sintaxis JSON o de conversión de parámetros de ruta/consulta los rechaza el extractor de Axum y pueden tener su formato predeterminado.

| Estado | Significado |
| --- | --- |
| `400` | JSON válido con campo, tipo o filtro inválido |
| `404` | El documento solicitado no existe |
| `409` | ID existente con contenido diferente o conflicto de revisión |
| `422` | Regla de negocio incumplida, como cantidad no válida |
| `503` | CouchDB no está disponible |
| `500` | Error interno o documento CouchDB incompatible/malformado |

Ejemplo de validación (422):

```json
{"error":"El precio debe ser un número válido y no negativo"}
```

## Telemetría

Todas las solicitudes HTTP se registran con identificador de correlación (también devuelto en `X-Request-Id`), método, ruta, estado y duración. `GET /metrics` expone contadores de solicitudes/5xx, histogramas de duración por método y ruta normalizada, y gauges `mercapos_process_threads` y `mercapos_process_resident_memory_bytes`. Para recolectar CPU, RAM e hilos de la API y CouchDB con evidencia CSV/LOG, use los scripts `mercapos_api/scripts/muestrear_recursos.py` y `prueba_carga.py`. Para que Prometheus recoja las métricas, añade un scrape job como este:

```yaml
scrape_configs:
  - job_name: mercapos-api
    metrics_path: /metrics
    authorization:
      type: Bearer
      credentials_file: /ruta/privada/mercapos-metrics-token
    static_configs:
      - targets: ["host.docker.internal:3000"]
```

La validación del inventario se serializa dentro de un proceso de la API; si se despliegan varias instancias escritoras, la coordinación de existencias requiere un mecanismo compartido además de las revisiones de CouchDB.
