# MercaPOS - PPI115 GT02

Punto de venta para equipos de gama baja con backend en Rust (Axum), persistencia
en Apache CouchDB, frontend Flutter y despliegue reproducible con Docker Compose.

## Arranque rápido con Docker

Requiere Docker con Compose. No hace falta instalar Rust, Flutter, Python ni
CouchDB.

```bash
docker compose up --build -d
```

En Windows también puede ejecutar `iniciar.bat`; en Linux/macOS, `./iniciar.sh`.
La primera ejecución compila la API y tarda varios minutos. Luego abra:

- Frontend: http://localhost:8080
- API: http://localhost:3000/health
- CouchDB (Fauxton): http://localhost:5985/_utils

Credenciales de la demo:

- Usuario: `usuario:demo-admin`
- Contraseña: `MercaPOS2026`

CouchDB admin de la demo: `admin` / `MercaPOSadmin2026` (ver `.env.example`).

## Estructura

```
mercapos_api/              Backend Rust/Axum, pruebas y scripts de metrología
mercapos-db/               Diseños, validaciones e instalador del esquema CouchDB
mercapos_flutter/          Frontend Flutter (fuente y build web precompilado)
mercapos_flutter/build/web Frontend listo para servir con nginx
deploy/                    Dockerfile, nginx y datos demo
docker-compose.yml         CouchDB + inicialización + API + datos demo + web
resultados_metrologia/     Evidencia de la Fase 4 (CSV, LOG, métricas y PDF)
```

## Fase 4: metrología y calidad

La instrumentación de la API expone `/metrics` con latencia por endpoint,
contadores HTTP, errores 5xx, hilos y memoria residente. La recolección es
reproducible con dos scripts de Python (solo biblioteca estándar):

```bash
cd mercapos_api

# Muestreo de CPU/RAM/hilos de la API y CouchDB
python3 scripts/muestrear_recursos.py \
  --api-pid <PID> --couchdb-container mercapos-couchdb \
  --duracion 40 --intervalo 1

# Carga concurrente y latencias p50/p95/p99
python3 scripts/prueba_carga.py \
  --api http://127.0.0.1:3000 \
  --usuario usuario:demo-admin --password MercaPOS2026 \
  --puesto puesto:demo --operaciones 200 --concurrencia 30

# En Docker
python3 scripts/muestrear_recursos.py \
  --api-container mercapos-api-1 --couchdb-container mercapos-couchdb-1 \
  --duracion 40 --intervalo 1
```

Los resultados se guardan en `resultados_metrologia/`. El informe consolidado
está en
[`resultados_metrologia/Informe_Metrologia_MercaPOS.pdf`](resultados_metrologia/Informe_Metrologia_MercaPOS.pdf).
Los detalles de uso están en
[`mercapos_api/scripts/README.md`](mercapos_api/scripts/README.md) y el contrato
de métricas en [`mercapos_api/ENDPOINTS.md`](mercapos_api/ENDPOINTS.md).

## Pruebas y verificación

```bash
cd mercapos_api
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test -- --ignored

cd ../mercapos_flutter
flutter analyze
flutter test --platform chrome
flutter build web --release
```

## Detener y limpiar

```bash
docker compose down       # detiene servicios y conserva el volumen
docker compose down -v    # elimina también los datos de la demo
```

## Seguridad

No se versiona ningún `.env` real. Use `.env.example` como plantilla y mantenga
las credenciales locales fuera del repositorio. Los datos, cuentas y cobros de la
demo son ficticios.
