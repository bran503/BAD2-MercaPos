# Contrato para el frontend de MercaPOS

**Estado 2026-11-01:** hay rutas de autenticación, autorización y sincronización por lotes; la compilación, Clippy, pruebas unitarias y tres pruebas de integración con CouchDB aislado pasan. El backend limita login a cinco fallos por cuenta en 15 minutos y serializa el bootstrap dentro de la instancia escritora. Una venta física offline con stock central insuficiente se guarda como `aceptada_con_incidencia`, puede dejar existencia negativa y genera una incidencia para conciliación. `/metrics` requiere una sesión de administrador.

## Autenticación

| Método y ruta | Uso |
| --- | --- |
| `POST /auth/bootstrap` | Crea el primer puesto, administrador y credencial. Requiere `X-Bootstrap-Token` igual a `MERCAPOS_BOOTSTRAP_TOKEN` y solo funciona si no existe ninguna credencial |
| `POST /auth/login` | `{"usuario_id":"...","password":"..."}` devuelve `token`, `expira_en` (epoch en segundos) y `usuario` |
| `POST /auth/logout` | Revoca la sesión actual; responde 204 |
| `GET /auth/me` | Devuelve el usuario autenticado |
| `POST /auth/password` | `{"actual":"...","nueva":"..."}` cambia la contraseña del usuario autenticado |

Todas las rutas salvo `/`, `/health`, `/auth/login` y `/auth/bootstrap` exigen:

```
Authorization: Bearer <token>
```

Un token ausente, inválido, expirado o revocado devuelve 401. Un usuario inactivo o una operación fuera de su puesto devuelve 403. Cinco fallos de autenticación por cuenta dentro de 15 minutos bloquean login para esa cuenta durante 15 minutos; durante el bloqueo devuelve 429. El token es opaco; el servidor solo guarda su hash SHA-256 en documentos `sesion:`. Credenciales y sesiones se almacenan en una base CouchDB separada (`COUCHDB_AUTH_DATABASE`).

Roles:

- `administrador`: gestiona usuarios y productos de su propio puesto; no es un administrador global.
- `vendedor`: solo consulta y opera en su propio `puesto_id`, y no puede suplantar a otro `usuario_id`.

La contraseña se guarda con Argon2id en `credencial:{usuario_id}` dentro de `COUCHDB_AUTH_DATABASE`, separada del libro comercial. Las sesiones también se guardan allí y esa base no debe replicarse. El documento `usuario` no contiene contraseña ni hash.

## Operaciones

- El dispositivo debe crear un UUID por operación antes del primer envío y conservarlo en todos los reintentos. Mantenga la cola en almacenamiento persistente; no la borre por timeout o respuesta perdida.
- Envíe `POST /operaciones` con `id`, `tipo_operacion`, `puesto_id`, `usuario_id`, `dispositivo_id`, `fecha_local` RFC 3339 con zona y `renglones`. `usuario_id` debe ser el usuario autenticado. `id` de la API corresponde a `_id` de CouchDB. El servidor añade `recibido_en_epoch_ms` en UTC y conserva `nombre_producto` y `unidad` de cada producto como snapshot; el cliente no debe enviarlos ni cambiarlos al reintentar.
- Cantidad: máximo tres decimales y hasta 1,000,000 unidades por renglón. Precio o costo: máximo cuatro decimales y hasta USD 1,000,000 por unidad. Máximo 100 renglones. El servidor redondea cada subtotal positivo al centavo, mitad hacia arriba, y suma los centavos. El `total` se puede omitir; si se envía debe coincidir exactamente o la API devuelve 422. La respuesta incluye `total_centavos`.
- Una venta puede incluir `pago` (opcional): `{"metodo":"tarjeta","modo":"simulado","estado":"aprobado","autorizacion":"AUT-123456","fecha":"<RFC 3339 con zona>"}`. Solo se acepta en ventas, con `metodo` y `modo` fijos, `estado` en `aprobado`, `rechazado` o `cancelado`, y `autorizacion`/`fecha` no vacías. Si el pago contiene datos de tarjeta reales la API los ignora; el cliente nunca debe enviarlos. Un pago rechazado o cancelado no se envía: sin pago aprobado no hay venta.
- Ventas y mermas reducen stock; compras lo aumentan. Devolución `cliente` referencia una venta; devolución `proveedor` referencia una compra. `destino_devolucion: "no_vendible"` conserva la devolución sin aumentar stock vendible. Los ajustes con `conteo_id` requieren diferencias coincidentes con un conteo POS no consumido.
- `POST /movimientos` solo abre inventario inicial, con `usuario_id` de un administrador, `motivo: "inventario_inicial"`, cantidad positiva e ID estable con prefijo `mov:`. El segundo ID de apertura del mismo producto y puesto devuelve 409.
- `POST /conteos` exige `periodo_estudio`, `etapa` (`antes` o `despues`), `origen_registro` (`cuaderno` o `pos`) y cantidades contadas. En `pos`, el servidor calcula `cantidad_sistema` y guarda un corte; en `cuaderno`, envíe `cantidad_sistema` y `evidencia`.
- `GET /productos` exige `puesto_id`; cada producto incluye `puesto_id` y `revision`. Las respuestas de puesto y usuario también incluyen `revision`. Los `PUT` de estos tres recursos exigen la revisión leída para detectar edición obsoleta. `GET /puestos/{id}/existencias` incluye `existencia_mil` exacta y `existencia` decimal para visualización.

## Sincronización offline

Use `POST /sync/operaciones` para enviar la cola:

```json
{"operaciones": [{"id": "operacion:...", "tipo_operacion": "venta", "...": "..."}]}
```

Máximo 100 operaciones por lote. Si puede guardar todas las incidencias requeridas, la respuesta es 200 y contiene un resultado por operación. Si falla la persistencia de una incidencia, devuelve error y el cliente debe conservar el lote para reintentar:

```json
{
  "resultados": [
    {"operacion_id": "operacion:1", "estado": "aceptada", "mensaje": "...", "incidencia_id": null},
    {"operacion_id": "operacion:2", "estado": "aceptada_con_incidencia", "mensaje": "El precio registrado ...", "incidencia_id": "incidencia:operacion:2"},
    {"operacion_id": "operacion:3", "estado": "rechazada", "mensaje": "Existencia insuficiente ...", "incidencia_id": "incidencia:operacion:3"}
  ],
  "aceptadas": 1,
  "con_incidencia": 1,
  "rechazadas": 1
}
```

Significado:

- `aceptada`: operación nueva o reintento idéntico ya guardado. Es seguro eliminarla de la cola.
- `aceptada_con_incidencia`: la operación se guardó y requiere revisión. Ocurre si el precio registrado difiere del actual, o si una venta offline se aceptó pese a que el stock central era insuficiente. En el segundo caso la existencia puede quedar negativa hasta la conciliación.
- `rechazada`: validación, autorización, conflicto de ID o falta de stock en operaciones distintas de una venta offline. Se guarda una incidencia durable con el motivo y el código HTTP. Corrija la causa y reenvíe la operación si corresponde. Una venta enviada por `/operaciones` (en línea) sigue rechazándose si no hay stock; la aceptación con existencia negativa aplica solo a `/sync/operaciones`.

Incidencias:

| Método y ruta | Uso |
| --- | --- |
| `GET /sync/incidencias?puesto_id=&resuelta=false` | Lista incidencias del puesto autenticado |
| `PUT /sync/incidencias/{operacion_id}/resolver` | Admin marca la incidencia como resuelta; cuerpo opcional `{"nota":"..."}` |

Cada incidencia se guarda de forma durable con `_id = incidencia:{operacion_id}` (o un hash si el ID no es seguro). Los reintentos incrementan `intentos` y actualizan `actualizada_en`; no se pierde el registro.

## Respuestas y red

Una creación nueva responde 201; un reintento idéntico ya guardado responde 200; mismo ID con contenido distinto responde 409. Un total incorrecto o una regla de negocio incumplida devuelve 422. Los errores usan `{"error":"..."}`; una venta en línea sin stock añade `"codigo":"stock_insuficiente"`.

CORS se configura con `MERCAPOS_CORS_ORIGINS` (lista separada por comas). Para Flutter nativo, CORS no reemplaza una dirección de red alcanzable; use la IP del equipo que ejecuta la API.
