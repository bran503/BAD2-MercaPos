# Auditoría y migración de MercaPOS v1 a v2

La herramienta `tools/migrar.py` **lee el origen y genera un reporte por defecto**. No modifica la base original. Requiere Python 3.12 y usa solo la biblioteca estándar.

## Ensayo con los fixtures

Desde la carpeta `BD2`:

```bash
python3 mercapos-db/tools/migrar.py \
  --fixtures mercapos-db/http/02-documentos.http mercapos-db/http/06-desconexion.http mercapos-db/http/09-operaciones-adicionales.http \
  --report-dir /tmp/mercapos-auditoria-prueba
python3 -m unittest discover -s mercapos-db/tools -p 'test_*.py'
node mercapos-db/tools/test_validacion.js
```

El reporte contiene `origen.json`, `propuesto.json` y `reporte.json`. El directorio debe ser privado (`chmod 700`); los archivos contienen todos los datos de negocio. El fixture corregido produce 63/29 unidades. La copia de `fixtures/legacy` conserva la referencia rota original: la auditoría la informa y bloquea la aplicación. La suite local también prueba la actualización idempotente del instalador y `test_validacion.js` evalúa la misma función JavaScript que CouchDB almacena.

## Captura de una base existente

Antes de capturar, identifique instancia, base y volumen. Detenga las escrituras comerciales del origen. La herramienta compara `update_seq` antes y después de la lectura; si cambia, aborta. No interpreta esa secuencia como entero. Use un directorio de reporte privado:

```bash
install -d -m 700 /ruta/privada/reporte
COUCHDB_USERNAME=admin COUCHDB_PASSWORD='clave-local' \
  python3 mercapos-db/tools/migrar.py \
  --source-url http://127.0.0.1:5985 --source-db mercapos \
  --report-dir /ruta/privada/reporte
```

El reporte incluye manifiesto SHA-256 por documento, saldos, mapeo de productos por puesto, anomalías y conflictos detectados. En una lectura viva también guarda `seguridad_origen.json` e `indices_origen.json`. Si hay ramas en conflicto, se informa y se bloquea el cambio: el snapshot de revisiones ganadoras **no es un respaldo completo de las ramas**. Conserve aparte la configuración de replicación, las credenciales y una copia consistente del volumen según el procedimiento de CouchDB que use su instalación.

La herramienta concilia cada renglón comercial con un único movimiento por operación, producto y cantidad firmada. Reconoce movimientos con ambos formatos de ID, informa duplicados y no inventa movimientos faltantes. Las aperturas sin operación se convierten en eventos `apertura_inventario`. Los movimientos huérfanos de otro tipo bloquean la aplicación. Un producto sin puesto comprobable también bloquea el cambio.

## Aplicación de prueba y cambio de configuración

Solo después de revisar un reporte sin anomalías, prepare una **base destino nueva** y use `--apply` con `--dest-url` y `--dest-db` distintos del origen. La escritura es reanudable: al repetirla compara cada documento existente con el propuesto y detiene diferencias. La herramienta instala vistas, validador, índices y `_security` en el destino. Usa los artefactos compartidos de `design_docs/`; el validador permite el tipo `incidencia` que necesita la API y conserva los conteos legacy como versión 1. No cambia automáticamente `COUCHDB_DATABASE` del backend.

```bash
COUCHDB_USERNAME=admin COUCHDB_PASSWORD='clave-de-prueba' \
  python3 mercapos-db/tools/migrar.py \
  --source-url http://127.0.0.1:5985 --source-db mercapos_test_origen \
  --dest-url http://127.0.0.1:5985 --dest-db mercapos_test_destino \
  --report-dir /ruta/privada/reporte-prueba --apply
```

Antes de cambiar la aplicación, compare saldos por puesto y producto en `reporte.json`, `/_design/inventario/_view/existencias_mil?group=true` y la API. Repita la migración y reenvíe las dos ventas legacy: el saldo no debe cambiar. Pruebe una restauración en otra base de ensayo. El usuario reportó desde otra sesión pruebas CouchDB de autenticación/sync y paginación; la migración de una base live y la restauración siguen sin ejecutarse.

La vuelta atrás previa al cambio de aplicación consiste en conservar la configuración original. Después de admitir escrituras en el destino, **no** apunte simplemente a la copia antigua: primero exporte y concilie esas operaciones nuevas o perderá ventas. Los conteos legacy quedan marcados como versión 1; la herramienta no inventa etapa de estudio, corte verificable ni identidad autenticada.

## Restricciones actuales

La herramienta de migración no resuelve automáticamente conflictos de revisión. Para la base que usará la API, cree una cuenta de datos con un rol dedicado, después configure el backend con esa cuenta y no con admin:

Para una base v2 vacía que no proviene de migración, créela primero y ejecute como administrador `python3 mercapos-db/tools/instalar_esquema.py --database <base-v2>`. El instalador requiere que la base ya exista, usa `_rev` para actualizar diseños y crea los mismos índices que la migración. No crea credenciales, no cambia `_security` y no inserta datos comerciales. Luego ejecute `preparar_cuenta_api.py` como se muestra abajo.

```bash
COUCHDB_ADMIN_USERNAME=admin COUCHDB_ADMIN_PASSWORD='clave-admin' \
API_COUCHDB_USERNAME=mercapos_api API_COUCHDB_PASSWORD='clave-servicio' \
COUCHDB_DATABASE=mercapos \
python3 mercapos-db/tools/preparar_cuenta_api.py
```

El script crea `mercapos_auth` si todavía no existe y añade el rol `mercapos_writer` en `_security` de las bases de negocio y autenticación; conserva los administradores y otros miembros existentes. Si encuentra una cuenta preexistente con roles diferentes, se detiene para revisión. No almacena las claves. Configure la API con `COUCHDB_AUTH_DATABASE=mercapos_auth`; no replique esa base. La cuenta puede leer y escribir documentos en ambas bases, por lo que el aislamiento por usuario/puesto debe aplicarlo la API.

La verificación integral requiere CouchDB aislado y acceso de compilación a las dependencias Rust.
