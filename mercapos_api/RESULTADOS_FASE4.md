# Evidencia de la Fase 4

Fecha de medición: 2026-11-01. Entorno de demostración en un equipo AMD Ryzen 5 7520U, 8 CPU lógicas y 7 GiB de RAM. CouchDB 3.3 se ejecutó en Docker; Rust API y cliente de carga se ejecutaron en el host.

## Construcción del artefacto

- **Backend:** Rust/Axum con capas presentación, aplicación, dominio e infraestructura; cliente asíncrono Reqwest para la API HTTP de CouchDB y consultas Mango.
- **Base documental:** `mercapos` usa los diseños `inventario` y `validacion_v2`, once índices Mango y la vista `existencias_mil`. La revisión confirmó dos productos y saldos de 49 000 y 29 000 milésimas en el puesto de demo.
- **Cliente web:** Flutter/Dart usa IndexedDB mediante `sembast_web`. La cola de operaciones se envía a Rust; el navegador no se conecta directamente a CouchDB.

## Mantenibilidad y compilación

- `cargo fmt --check`: aprobado.
- `cargo clippy --all-targets -- -D warnings`: aprobado.
- `cargo test`: 30 pruebas locales aprobadas; 3 pruebas de integración se ejecutan explícitamente contra bases aisladas.
- `flutter analyze`: sin problemas.
- `flutter test --platform chrome`: 6 pruebas aprobadas.
- `flutter build web --no-wasm-dry-run`: compilación aprobada. La última compilación incremental tomó 3.4 s; esta duración es una medición de compilación en caché, no una comparación entre compilaciones limpias.

## Prueba concurrente de sincronización

Se creó `mercapos_test_fase4` y su base de autenticación separada. Se sembraron documentos ficticios y se enviaron operaciones `compra` por `POST /sync/operaciones`, con IDs únicos y 100 solicitudes clientes concurrentes. La prueba no usó ni escribió en la base de demo.

| Métrica | Resultado |
|---|---:|
| Operaciones enviadas | 200 |
| Operaciones aceptadas | 200 (100 %) |
| Errores/rechazos | 0 |
| Duración total | 4.952 s |
| Throughput | 40.39 operaciones/s |
| Latencia cliente p50 | 2291.59 ms |
| Latencia cliente p95 | 2503.39 ms |
| Latencia cliente p99 | 2508.05 ms |
| CPU máxima observada de la API | 214.3 % de un núcleo |
| Memoria residente máxima de la API | 52.5 MiB |
| Hilos máximos observados de la API | 10 |

La latencia incluye el trayecto HTTP y la persistencia de CouchDB. La muestra indica que el backend completa la carga y que la escritura concurrente se serializa; no demuestra una meta de latencia inferior a 15 ms. Una prueba separada de 100 operaciones con 25 solicitudes concurrentes logró 42.68 operaciones/s y 0 rechazos.

CouchDB se observó mediante `docker stats` durante una carga más pequeña: dos muestras fueron 100.25 % y 20.58 % de CPU, con memoria entre 58.18 y 58.37 MiB. La muestra es demasiado breve para tomarla como perfil estable; se conserva como referencia inicial. Una muestra posterior en reposo registró 0.52 % y 63.16 MiB.

## Alcance de esta evidencia

La telemetría del backend registra duración total por endpoint, estado, método y ruta normalizada; la carga anterior aporta percentiles de latencia y aceptación a nivel de cliente. El muestreo del proceso añade CPU, RSS e hilos de Rust. Las mediciones describen este equipo y esta configuración local. Para afirmar resultados generales o cumplir metas académicas de carga, hay que repetirlas bajo el hardware, tamaño de muestra y condiciones de red que establezca el protocolo experimental. No se midieron pérdida de paquetes, ancho de banda limitado, latencia de red ni restauración desde respaldo en esta corrida.

## Instrumentación ampliada y scripts de metrología (2026-11-13)

Se añadieron histogramas Prometheus de latencia por método y ruta normalizada, un gauge de hilos reales y otro de memoria residente del proceso Rust (`src/presentation/telemetry.rs`). Se crearon dos scripts reproducibles en `mercapos_api/scripts/`: `muestrear_recursos.py` (CPU, RSS e hilos de la API y de CouchDB) y `prueba_carga.py` (carga concurrente de transacciones, latencias, throughput y capturas de `/metrics`).

Corrida documentada sobre una base aislada `mercapos_medicion` y una segunda instancia de la API en el puerto 3001: 200 compras concurrentes con 30 hilos, 200 aceptadas, 0 errores 5xx, 6.061 s, 33.00 operaciones/s, latencia cliente p50 884.53 ms, p95 922.88 ms y p99 969.52 ms. La telemetría del servidor registró 200 solicitudes, 167 508 ms acumulados y 837.54 ms de media por transacción. Recursos: API con CPU media 9.05 % (máx. 50.90 %), RSS medio 44.10 MiB y 9–10 hilos; CouchDB con CPU media 34.84 % (máx. 229.91 %) y memoria media 93.01 MiB. El informe completo, los CSV y las exposiciones de métricas están en `resultados_metrologia/`; el PDF es `resultados_metrologia/Informe_Metrologia_MercaPOS.pdf`.

Calidad y proceso: `cargo fmt --check`, Clippy, 32 pruebas, tres compilaciones consecutivas, `flutter analyze`, 8 pruebas Chrome y build web release, todos correctos. Los tiempos de compilación son incrementales y no sustituyen una comparación de compilaciones limpias.
