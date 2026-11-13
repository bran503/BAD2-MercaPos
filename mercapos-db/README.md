# MercaPOS: Base de datos Apache CouchDB (Fase 4)

> **Estado 2026-11-01:** el contenedor CouchDB está saludable. Se instalaron en `mercapos` los dos diseños v2 y los 11 índices Mango faltantes mediante `tools/instalar_esquema.py`; la verificación posterior confirmó que coinciden con los archivos versionados. La vista `existencias_mil` devolvió 49 000 para tomate y 29 000 para cebolla del puesto de demo. La cuenta de servicio tiene el rol `mercapos_writer` y las bases de negocio y autenticación están separadas. Pasaron nueve pruebas Python, el validador JavaScript y tres integraciones Rust con bases `mercapos_test_*`. No se ejecutó una migración ni una restauración de respaldo.

La configuración activa usa `mercapos` y `mercapos_auth` con la API Rust como único escritor comercial. Actualmente no existe `mercapos_central` ni hay tareas en `/_scheduler/docs`; las secciones de replicación bidireccional de esta guía documentan un ensayo anterior y no forman parte de la sincronización actual de Flutter. La web guarda una cola local y la envía a `POST /sync/operaciones`.

Este documento explica, paso a paso, cómo levantar y revisar la base de datos documental del proyecto **MercaPOS: evaluación de una arquitectura offline-first con Rust y Apache CouchDB**. Corresponde al componente de base de datos (integrante 3) de la Fase 4 y sirve como guía para que otro integrante pueda reproducir las pruebas.

### Resolver conflictos de catálogo con revisión explícita

`tools/resolver_conflicto.py` inspecciona las hojas de conflicto de un documento `producto`, `puesto` o `usuario`. La ejecución normal es una simulación de solo lectura. Revisa primero cada revisión con `?rev=<revision>` y acuerda manualmente cuál conservar; CouchDB no determina cuál precio o dato comercial es correcto. Después aplica solo en una base de laboratorio o durante una ventana controlada:

```bash
COUCHDB_USERNAME=admin COUCHDB_PASSWORD='...' \
python3 tools/resolver_conflicto.py --database mercapos_test_conflictos \
  --id 'producto:0042' --keep-rev '3-<revision>'
```

Tras revisar la salida y el contenido de la revisión seleccionada, repite con `--apply`. La herramienta reemplaza el tronco con el contenido elegido y elimina todas las hojas conflictivas observadas; si detecta cambios durante la resolución, se detiene y deja las hojas restantes para revisión. Rechaza `operacion`, conteos y movimientos por su carácter contable o histórico. Nunca ejecutes `--apply` sin respaldo, selección explícita y revisión humana del documento.

## 1. Alcance

El proyecto incluye una instancia de Apache CouchDB en Docker, documentos JSON del sistema, índices Mango, una vista para calcular existencias y herramientas para migrar y revisar conflictos. La replicación bidireccional entre dos bases pertenece al laboratorio histórico descrito más abajo.

Las pruebas usan datos ficticios y anonimizados. La "base local" (`mercapos`) simula el almacenamiento de un dispositivo y la "base central" (`mercapos_central`) simula el nodo central, ambas dentro de una misma instancia de CouchDB. Los resultados corresponden a condiciones de laboratorio y no reflejan mediciones en mercados reales.

## 2. Requisitos

- Windows 10/11 con WSL 2.
- Docker Desktop.
- Visual Studio Code con la extensión **REST Client** (`humao.rest-client`).
- Conexión a Internet la primera vez, para descargar la imagen `couchdb:3.3`.

## 3. Estructura del proyecto

```
mercapos-db/
├── docker-compose.yml
├── README.md
├── design_docs/                    vistas, validador e índices versionados
├── tools/instalar_esquema.py       instalador repetible de esquema v2
└── http/
    ├── 01-setup.http                  bases de datos
    ├── 02-documentos.http             puesto, usuarios, productos, inventario inicial y una venta
    ├── 03-indices-mango.http          índices
    ├── 04-vistas.http                 vista de existencias
    ├── 05-replicacion.http            replicación bidireccional
    ├── 06-desconexion.http            prueba de desconexión y recuperación
    ├── 07-conflictos.http             provocar y detectar un conflicto
    ├── 08-resolver-conflictos.http    resolver el conflicto
    ├── 09-operaciones-adicionales.http  compra, merma, devolución, conteo físico y ajuste
    └── 10-consultas.http              consultas Mango y de vista usadas por el backend
```

Cada archivo `.http` contiene peticiones que se ejecutan haciendo clic en **Send Request** sobre la petición deseada. No es necesario enviar a ciegas todas las peticiones: revisa primero la sección 4 para evitar repetir escrituras en una base que ya tiene datos.

## 4. Puesta en marcha

### 4.1 Iniciar CouchDB

1. Instala y abre Docker Desktop; espera a que indique **Engine running**.
2. Abre en VS Code la carpeta `mercapos-db`.
3. Abre una terminal de PowerShell en esa carpeta y ejecuta:

   Antes, copia `.env.example` a `.env`, define una contraseña privada y copia `http/.env.example` a `http/.env` con la misma clave y su variante codificada para URL.

   ```powershell
   docker compose up -d
   ```

4. Comprueba que el contenedor esté activo:

   ```powershell
   docker ps
   ```

   Debe aparecer `mercapos-couchdb` con estado `Up` y el puerto `5985->5984`.

5. En el navegador abre [http://localhost:5985/](http://localhost:5985/). Debe aparecer una respuesta JSON con `"couchdb": "Welcome"`.
6. Para abrir la interfaz visual de CouchDB (Fauxton), ve a [http://localhost:5985/_utils](http://localhost:5985/_utils) e inicia sesión con las credenciales locales indicadas abajo.

### 4.2 Primera configuración de una base vacía

Esta secuencia levanta el **laboratorio legacy** de replicación y escribe documentos directamente con REST Client. No la ejecutes en la base configurada para la API ni la combines con el validador v2.

En VS Code, abre `http/01-setup.http` y envía sus peticiones **una por una**, de arriba hacia abajo. Después ejecuta los archivos en este orden:

1. `02-documentos.http`: crea los documentos iniciales y los movimientos.
2. `03-indices-mango.http`: crea los índices para las consultas.
3. `04-vistas.http`: crea la vista de existencias.
4. `05-replicacion.http`: crea `mercapos_central` y las dos tareas continuas.
5. `06-desconexion.http`: demuestra registro local y sincronización posterior.
6. `09-operaciones-adicionales.http`: añade compra, merma, devolución, conteo y ajuste.
7. `07-conflictos.http` y luego `08-resolver-conflictos.http`: provocan y resuelven un conflicto de producto. Ejecuta sus solicitudes en orden, incluyendo primero los `GET` que capturan las revisiones actuales.
8. `10-consultas.http`: ejecuta las consultas de lectura para validar los índices y los datos.

En REST Client, usa **Send Request** para cada solicitud y revisa la respuesta antes de continuar. Algunas solicitudes de creación responden con `201 Created`; si ya existe el recurso pueden responder con `412` o `409`, según la solicitud. No repitas una escritura solo para “comprobar” que funciona.

### 4.3 Comprobar una instancia que ya tiene datos

Si el contenedor y las bases ya existen, **no repitas** las peticiones `PUT`, ni las pruebas de desconexión o conflictos. Para revisar sin cambiar datos:

1. Abre `http/01-setup.http` y envía únicamente `GET {{host}}/`.
2. En `http/05-replicacion.http`, envía únicamente `GET {{host}}/_scheduler/docs`.
3. En `http/03-indices-mango.http`, envía únicamente `GET {{host}}/mercapos/_index`.
4. Ejecuta las consultas de `http/10-consultas.http` y la consulta de existencias de `http/04-vistas.http`.
5. Para inspeccionar los documentos visualmente, abre Fauxton y selecciona `mercapos` o `mercapos_central`.

El estado esperado de cada replicación es `"state": "running"` y `"error_count": 0`. Las consultas Mango deben devolver resultados sin `warning`; el archivo `10-consultas.http` aún debe ejecutarse para comprobarlo. La vista de existencias ya se consultó después de registrar las operaciones adicionales y devolvió 63 unidades de `producto:0042` y 29 de `producto:0107` en `puesto:0001`.

### 4.4 Preparar una base nueva para la API Rust (v2)

Usa una base recién creada o el destino de una migración v2. No instales este validador sobre la fuente legacy: sus productos deben migrarse primero para recibir `puesto_id` y `schema_version: 2`. El instalador exige una base existente, usa cuenta admin, actualiza los documentos de diseño con `_rev`, crea los índices definidos y no cambia documentos comerciales ni `_security`.

Desde la raíz `BD2`, carga las variables privadas del componente y crea una base vacía de prueba (usa un nombre exclusivo):

```bash
set -a
. mercapos-db/.env
set +a
export COUCHDB_URL=http://127.0.0.1:5985
curl --fail --silent --show-error -u "$COUCHDB_ADMIN_USERNAME:$COUCHDB_ADMIN_PASSWORD" \
  -X PUT "$COUCHDB_URL/mercapos_test_v2"
python3 mercapos-db/tools/instalar_esquema.py --database mercapos_test_v2
COUCHDB_DATABASE=mercapos_test_v2 COUCHDB_AUTH_DATABASE=mercapos_test_auth_v2 \
  python3 mercapos-db/tools/preparar_cuenta_api.py
```

CouchDB puede responder `412` si la base de negocio ya existía; detente y verifica que sea una base desechable de prueba antes de continuar. `preparar_cuenta_api.py` crea la base de autenticación si falta y añade el rol de servicio a ambas bases. Configura el backend con el mismo usuario/clave de API y ambos nombres de base; después crea el administrador mediante el bootstrap protegido de la API. No insertes ventas, inventario ni credenciales con peticiones `.http` directas.

**Nota sobre el puerto.** CouchDB escucha en el 5984 dentro del contenedor, pero se publica en el **5985** del equipo anfitrión porque en el equipo de desarrollo el 5984 ya estaba ocupado. Si el 5984 está libre, puede usarse modificando `docker-compose.yml` y la variable `@host` de los archivos `.http`. Las URLs de replicación usan siempre el 5984, ya que se resuelven desde el interior del contenedor.

**Credenciales locales.** `COUCHDB_PASSWORD` se define fuera de los archivos versionables. Los `.http` la leen desde `http/.env` mediante REST Client; las URLs internas de replicación leen `COUCHDB_PASSWORD_URLENCODED`. El puerto 5985 se vincula a `127.0.0.1` en el archivo Compose.

**Apagar conservando datos.** Usa `docker compose down`. El volumen persistente se mantiene. Para repetir pruebas, crea un proyecto Compose aislado y una base con nombre exclusivo de pruebas; no limpies el volumen de trabajo.

## 5. Modelo de documentos

En la base de negocio los documentos se distinguen por `tipo`; las credenciales y sesiones viven en otra base, que no se replica. En el modelo v2 la vista calcula saldos a partir de operaciones canónicas y aperturas; los movimientos legacy se leen solo para compatibilidad.

### 5.1 `puesto` (negocio y ubicación)

| Campo | Tipo | Obligatorio | Descripción |
|---|---|---|---|
| `_id` | texto | sí | `puesto:<número>` |
| `tipo` | texto | sí | `"puesto"` |
| `nombre` | texto | sí | Nombre del negocio |
| `mercado` | texto | sí | Mercado donde opera |
| `ubicacion` | texto | sí | Ciudad y país |
| `propietario_usuario_id` | texto | sí | `usuario:<número>` del propietario |
| `activo` | booleano | sí | |

### 5.2 `usuario` (quién vende, registra entradas o realiza ajustes)

Las credenciales (contraseñas) no se almacenan en este documento: se gestionan por separado.

| Campo | Tipo | Obligatorio | Descripción |
|---|---|---|---|
| `_id` | texto | sí | `usuario:<número>` |
| `tipo` | texto | sí | `"usuario"` |
| `nombre` | texto | sí | |
| `rol` | texto | sí | `vendedor` o `administrador` |
| `puesto_id` | texto | sí | `puesto:<número>` |
| `activo` | booleano | sí | |

### 5.3 `producto` (catálogo)

| Campo | Tipo | Obligatorio | Descripción |
|---|---|---|---|
| `_id` | texto | sí | `producto:<código>` |
| `tipo` | texto | sí | `"producto"` |
| `nombre` | texto | sí | |
| `categoria` | texto | sí | |
| `precio_unitario` | número | sí | Precio de venta |
| `unidad` | texto | sí | Por ejemplo `libra` |
| `activo` | booleano | sí | |
| `actualizado_en` | fecha ISO 8601 | sí | Necesario para resolver conflictos por fecha más reciente |
| `actualizado_por` | texto | sí | `usuario:<número>` |

### 5.4 `operacion` (ventas, compras, mermas, devoluciones y ajustes)

| Campo | Tipo | Obligatorio | Descripción |
|---|---|---|---|
| `_id` | texto | sí | `operacion:<dispositivo>:<fecha>` (por ejemplo `operacion:disp03:20261020T143207`) |
| `tipo` | texto | sí | `"operacion"` |
| `tipo_operacion` | texto | sí | `venta`, `compra`, `merma`, `devolucion` o `ajuste` |
| `puesto_id`, `usuario_id`, `dispositivo_id` | texto | sí | Dónde, quién y desde qué dispositivo |
| `fecha_local` | fecha ISO 8601 | sí | Fecha y hora local del dispositivo |
| `recibido_en_epoch_ms` | entero Unix en milisegundos | API nueva | Instante UTC asignado por el servidor; los datos migrados conservan null/ausente si no existía evidencia fiable |
| `renglones[]` | arreglo | sí | Ver tabla de renglones |
| `total` | número | venta, compra, devolución | |
| `motivo` | texto | merma, ajuste | |
| `operacion_origen_id` | texto | devolución | Venta que se devuelve |
| `conteo_id` | texto | ajuste derivado de conteo | Conteo físico que lo originó |

Campos de cada renglón: `producto_id`, `nombre_producto` y `unidad` como snapshot del catálogo al aceptar la operación, `cantidad`, y según el tipo `precio_unitario` (venta, devolución) o `costo_unitario` (compra). Registros históricos pueden no tener snapshots y no se completan inventando valores. En venta, compra, merma y devolución la `cantidad` es positiva y el signo lo aporta el movimiento; en un **ajuste** la `cantidad` lleva signo (negativa si se descuenta).

### 5.5 `movimiento` (base del cálculo de existencias)

| Campo | Tipo | Obligatorio | Descripción |
|---|---|---|---|
| `_id` | texto | sí | `mov:<dispositivo>:<fecha>:r<n>:<producto>`, donde `r<n>` es el número de renglón |
| `tipo` | texto | sí | `"movimiento"` |
| `producto_id` | texto | sí | |
| `cantidad` | número | sí | Positiva para entradas, negativa para salidas |
| `motivo` | texto | sí | `inventario_inicial`, `venta`, `compra`, `merma`, `devolucion` o `ajuste` |
| `operacion_id` | texto | salvo `inventario_inicial` | Operación que lo originó |
| `dispositivo_id`, `fecha_local` | texto | sí | |

Cada renglón de una operación genera su propio movimiento. Incluir `r<n>` en el `_id` evita colisiones si un mismo producto aparece en dos renglones.

### 5.6 `conteo_fisico`

| Campo | Tipo | Obligatorio | Descripción |
|---|---|---|---|
| `_id` | texto | sí | `conteo:<dispositivo>:<fecha>` |
| `tipo` | texto | sí | `"conteo_fisico"` |
| `puesto_id`, `usuario_id`, `dispositivo_id`, `fecha_local` | texto | sí | |
| `renglones[]` | arreglo | sí | `producto_id`, `cantidad_sistema`, `cantidad_contada`, `diferencia` |

### 5.7 Criterios de diseño

- Los documentos de `02`, `06` y `09` representan el esquema del laboratorio legacy. En v2, una operación aceptada es el evento canónico único; sus impactos de inventario están dentro del mismo documento. La vista calcula los movimientos proyectados y no requiere documentos de movimiento persistidos para cada renglón.
- Operaciones y conteos aceptados son inmutables. Una corrección se registra mediante un evento posterior. Las incidencias de sincronización sí se actualizan para guardar su resolución; el validador admite esa transición.
- El instalador v2 valida tipo, prefijos, campos requeridos, cantidades, impacto contable, incidencias y conteos. Referencias entre documentos se validan en Rust, porque un validador CouchDB no puede hacer joins.

## 6. Procedimiento de configuración

### 6.1 Bases de datos (`01-setup.http`)
Crea `_users` y `_replicator` (necesarias en una instalación de un solo nodo), la base legacy `mercapos` y la base separada `mercapos_auth`. La lista de `GET {{host}}/_all_dbs` incluye al menos esos cuatro nombres.

### 6.2 Documentos de ejemplo (`02-documentos.http`)
Crea un puesto, dos usuarios, dos productos, el inventario inicial y una venta de dos renglones con sus movimientos. `GET {{host}}/mercapos/_all_docs` debe mostrar los documentos con revisiones que comienzan con `1-`.

### 6.3 Índices Mango (`design_docs/indices.json` y `03-indices-mango.http`)

| Índice | Campos | Consulta que sirve |
|---|---|---|
| `idx-tipo-operacion-fecha` | `tipo`, `tipo_operacion`, `fecha_local` | Ventas, mermas, compras, devoluciones y ajustes por rango de fechas |
| `idx-tipo-producto` | `tipo`, `producto_id` | Movimientos de un producto |
| `idx-tipo-activo` | `tipo`, `activo` | Catálogo de productos activos |
| `idx-tipo-fecha` | `tipo`, `fecha_local` | Conteos físicos por fecha |
| `idx-operacion-puesto-fecha` | `tipo`, `puesto_id`, `fecha_local` | Operaciones del puesto |
| `idx-operacion-schema-puesto` | `tipo`, `schema_version`, `puesto_id` | Operaciones v2 para la proyección de inventario |
| `idx-movimiento-puesto-producto` | `tipo`, `puesto_id`, `producto_id` | Filtro combinado de movimientos |
| `idx-apertura-puesto-producto` | `tipo`, `schema_version`, `puesto_id`, `producto_id` | Aperturas v2 por producto y puesto |
| `idx-catalogo-puesto-activo` | `tipo`, `puesto_id`, `activo` | Productos y usuarios del puesto |
| `idx-conteo-puesto-fecha` | `tipo`, `puesto_id`, `fecha_local` | Conteos v2 del puesto |
| `idx-incidencia-puesto-resuelta` | `tipo`, `puesto_id`, `resuelta` | Incidencias del puesto |

Los once índices oficiales están en `design_docs/indices.json`; `instalar_esquema.py` y `migrar.py` usan esa misma lista. Cada uno declara `ddoc` y nombre estables. `10-consultas.http` mantiene `use_index` para los cuatro índices legacy de sus fixtures. En una base v2, `_index` lista los índices oficiales más los índices internos del sistema y puede incluir otros previamente creados; no es necesario borrar esos índices.

### 6.4 Vista de existencias (`04-vistas.http`)
Mango no suma, por lo que las existencias se calculan con una vista map/reduce en `_design/inventario`:

La definición completa está versionada en `design_docs/inventario_views.json`. Publica las vistas `existencias` y `existencias_mil`: ambas leen movimientos legacy, eventos canónicos v2 y aperturas v2, usando escala decimal o milésimas. La reducción es `_sum`; `group=true` entrega una fila por puesto y producto. Los datos del laboratorio legacy producen 63/29 al cargar los archivos indicados una sola vez.

### 6.5 Replicación (`05-replicacion.http`)
La replicación de CouchDB opera en un solo sentido, por lo que se definen dos tareas continuas en `_replicator`: `local-a-central` y `central-a-local`. `GET {{host}}/_scheduler/docs` debe mostrarlas con `"state": "running"` y `"error_count": 0`. Los documentos de diseño (índices y vistas) también se replican.

### 6.6 Desconexión y recuperación (`06-desconexion.http`)
1. Se leen las revisiones vigentes de las tareas y se eliminan, simulando un corte.
2. Se registra una venta y su movimiento en la base local (`201 Created`).
3. La misma venta consultada en la base central da `404 Not Found`: se registró sin conexión.
4. Se recrean las tareas de replicación.
5. Tras 5 a 10 segundos la consulta devuelve `200 OK` y la vista de existencias del central refleja el cambio: 45 unidades de `producto:0042` en `puesto:0001`, antes de registrar las operaciones adicionales.

Aporta evidencia para los objetivos OE4 y OE5 y el requerimiento RF4.

### 6.7 Conflictos de sincronización (`07` y `08`)
Un conflicto ocurre cuando dos réplicas modifican el mismo documento mientras no hay replicación:

1. Con la replicación detenida, se modifica `producto:0042` en la base local (precio 1.50) y en la central (precio 1.35), ambas a partir de la misma revisión.
2. Al reactivar la replicación, CouchDB conserva las dos revisiones, elige una ganadora de forma determinista (la misma en ambas bases) y registra la otra en `_conflicts` (`GET .../producto:0042?conflicts=true`).
3. `_design/conflictos` incluye una vista que lista los documentos con conflictos.
4. `08` lee la revisión perdedora desde `_conflicts` y la elimina con `DELETE ...?rev=<revisión>`; si la ganadora no es el valor correcto, primero se escribe el valor correcto sobre la ganadora (paso 3B). La eliminación se replica y la vista de conflictos queda vacía.

**La ganadora de CouchDB no se elige por ser la más reciente ni la correcta.** La regla de resolución debe definirse a nivel de aplicación. Propuesta para acordar con los integrantes 1 (backend) y 2 (frontend), pendiente de confirmación:

| Documento | Regla propuesta |
|---|---|
| `producto` | El catálogo solo lo edita el administrador y **prevalece el nodo central**. Si se prefiere "la fecha más reciente", se compara `actualizado_en` |
| `operacion`, `movimiento`, `conteo_fisico` | No se editan; no se esperan conflictos |
| Cualquier otro caso | Revisión manual con la vista `conflictos/lista` |

### 6.8 Operaciones adicionales (`09-operaciones-adicionales.http`)
Registra una compra, una merma, una devolución, un conteo físico y el ajuste derivado, cada uno con su movimiento. Esta prueba ya se ejecutó: los documentos de operación, movimiento y conteo quedaron creados y se verificó la vista de existencias agrupada, que devolvió 63 unidades de tomate (`producto:0042`) y 29 de cebolla (`producto:0107`) en `puesto:0001`. Cronología del tomate: 50 − 2 + 20 − 2 + 1 − 3 = 64 al momento del conteo (18:00 del 24/10); el ajuste de −1 deja 63.

### 6.9 Consultas (`10-consultas.http`)
Contiene seis consultas Mango legacy y una consulta de la vista. Las peticiones especifican los cuatro índices legacy. La ejecución de comprobación del 27 de octubre de 2026 devolvió `HTTP 200 OK` sin advertencias en las respuestas compartidas. Esto valida los fixtures de laboratorio; las consultas v2 de la API usan sus índices de puesto y esquema descritos arriba.

## 7. Observaciones para la reproducción

- Las revisiones (`_rev`) **no están escritas a mano**: los archivos `06`, `07` y `08` las obtienen con una petición `GET` previa (`# @name`) y la reutilizan. Hay que ejecutar esos `GET` de cada archivo antes del `PUT` o `DELETE` que los usa, en la misma sesión de VS Code.
- Volver a enviar un `PUT` ya ejecutado devuelve `409 Conflict`. No es un fallo: el documento ya existe.
- Si la replicación tarda en reflejarse, esperar entre 5 y 10 segundos y repetir la consulta.
- Si se reutiliza una base anterior, audítala y migra hacia una base nueva según `MIGRACION.md`; conserva el volumen original.
- Para apagar el entorno sin eliminar datos: `docker compose down` (sin la opción `-v`).

## 8. Limitaciones

- `mercapos` y `mercapos_central` residen en la misma instancia de CouchDB, por lo que no se incluye latencia ni pérdida de paquetes reales. Las condiciones de red adversas se evaluarán en la fase de pruebas con `tc netem`.
- El cliente offline debe persistir su cola local y enviar operaciones a Rust; la base final no se replica directamente desde el cliente.
- La migración de la base operativa y la restauración de respaldo aún deben probarse sobre bases temporales antes de cambiar la configuración real.
- El cursor público de paginación corresponde a la API; CouchDB ya recorre sus bookmarks internamente.

## 9. Estado del componente y coordinación pendiente

La base dispone de vistas e índices v2 en `design_docs/`, validador para operación, catálogo, movimientos, conteos e incidencias, instalador repetible y auditoría/migración idempotente. Las consultas legacy de `10-consultas.http` se ejecutaron el 27 de octubre de 2026 y devolvieron `HTTP 200 OK` sin advertencias en las respuestas compartidas. El usuario también reportó que la prueba aislada CouchDB con 25 registros pasó en otra sesión de Codex.

Como comprobación complementaria, la consulta 3 puede volver a ejecutarse con `tipo_operacion` igual a `compra` y `ajuste`; la respuesta compartida para esa consulta corresponde a `devolucion`. Antes de integrar con el backend y el frontend, el equipo también debe confirmar los campos JSON definitivos, los endpoints y el flujo de almacenamiento/sincronización offline. La regla de resolución de conflictos aplicada en la prueba conserva el valor central de $1.35 para `producto:0042`. La prueba de desconexión detiene la tarea de replicación entre dos bases de la misma instancia y no simula una caída real de red.

Para entregar evidencias, guarda capturas o respuestas de: Fauxton mostrando ambas bases, `/_scheduler/docs` con las dos tareas activas, `/_index`, las siete consultas de lectura y la vista de existencias. No incluyas contraseñas en capturas que vayas a publicar.

## 10. Resultados históricos del laboratorio y límites

### 10.1. Funcionalidades implementadas y comprobadas

- **Configuración de CouchDB:** se dispone de las bases de datos `mercapos` y `mercapos_central`.
- **Índices Mango:** se crearon los índices definidos para facilitar las consultas de operaciones, productos y registros por tipo, estado y fecha.
- **Replicación bidireccional:** se configuraron las tareas de sincronización entre las bases de datos local y central; ambas se comprobaron en estado `running`, sin errores ni cambios pendientes.
- **Prueba de sincronización posterior a una desconexión simulada:** se pausó la replicación, se registraron documentos en la base local y posteriormente se reanudó la sincronización. Se comprobó la llegada de los documentos a la base central.
- **Detección y resolución de conflictos (`07-conflictos.http` y `08-resolver-conflictos.http`):** se modificó concurrentemente `producto:0042` en las dos bases, con precios de $1.50 en local y $1.35 en central. Al reanudar la replicación, CouchDB expuso la revisión perdedora en `_conflicts` y la vista `_design/conflictos/_view/lista` identificó el documento afectado. Se eliminó únicamente la revisión perdedora y se conservó el precio central de $1.35, según la regla definida para esta prueba. Después se comprobó que ambos nodos tenían la misma revisión, que el documento ya no incluía `_conflicts` y que la vista devolvía `total_rows: 0` y `rows: []`.
- **Operaciones adicionales (`09-operaciones-adicionales.http`):** se registraron y comprobaron la compra, la merma, la devolución, el conteo físico y el ajuste, incluidos sus movimientos. La consulta `GET /mercapos/_design/inventario/_view/existencias?group=true` devolvió `63` para la clave `["puesto:0001", "producto:0042"]` y `29` para la clave `["puesto:0001", "producto:0107"]`.
- **Consultas (`10-consultas.http`):** el 27 de octubre de 2026 las seis consultas Mango y la vista de existencias devolvieron `HTTP 200 OK`. Las respuestas compartidas no incluyen `warning` de índices. Se comprobaron dos ventas de `disp03`, una merma, una devolución, siete movimientos de `producto:0042`, dos productos activos, un conteo físico y las existencias finales de 63 y 29 unidades.

### 10.2. Comprobaciones complementarias

La validación principal de lectura del archivo `10-consultas.http` está completada. La consulta 3 se ejecutó con `tipo_operacion: "devolucion"`; si se necesita evidencia separada de todos los tipos que admite, falta repetir esa misma consulta con `compra` y `ajuste`. Esas variantes no deben presentarse como probadas hasta revisar sus respuestas.

Las operaciones adicionales, la vista de existencias final, las seis consultas Mango en su configuración compartida y la resolución del conflicto de `producto:0042` ya se validaron. La prueba de desconexión fue una pausa de la replicación, no una caída real de red.

### 10.3. Limitaciones de las pruebas

La prueba de desconexión realizada consistió en pausar temporalmente la replicación. Por tanto, demuestra el comportamiento de sincronización posterior, pero no sustituye una prueba de desconexión real de la red.

Además, las bases local y central se encuentran en la misma instancia de CouchDB. La evaluación de fallos de conectividad reales, latencia y pérdida de paquetes queda pendiente de pruebas adicionales.

### 10.4. Criterios para cerrar la validación

La detección y resolución de conflictos se considera validada: la revisión central de `producto:0042` con precio de $1.35 se conservó, ambos nodos quedaron sincronizados y la vista de conflictos terminó vacía.

La validación principal de `09-operaciones-adicionales.http` y `10-consultas.http` está completada. Las siete peticiones de lectura respondieron `HTTP 200 OK`; no se observó el aviso `No matching index found` en las respuestas compartidas y la vista confirmó 63 unidades de `producto:0042` y 29 de `producto:0107` en `puesto:0001`. Para una evidencia más completa, puede anexarse el resultado de la consulta 3 con `tipo_operacion` igual a `compra` y `ajuste`, ya que las respuestas recibidas hasta ahora corresponden a `devolucion`. Las limitaciones sobre desconexión real, latencia y pérdida de paquetes deben mantenerse explícitas.



