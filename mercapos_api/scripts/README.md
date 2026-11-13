# Scripts de metrología

Ambos scripts usan solo la biblioteca estándar de Python 3 y escriben CSV/LOG en
`resultados_metrologia/` (configurable con `--salida`).

## Muestreo de recursos

```bash
python3 scripts/muestrear_recursos.py \
  --api-pid <PID> --couchdb-container mercapos-couchdb \
  --duracion 40 --intervalo 1
```

En Docker use `--api-container mercapos-api-1 --couchdb-container mercapos-couchdb-1`.
Genera `recursos_*.csv` y `recursos_*.log` con CPU, RSS, hilos y memoria de CouchDB
(mínimo, media y máximo).

## Carga concurrente

```bash
python3 scripts/prueba_carga.py \
  --api http://127.0.0.1:3000 \
  --usuario usuario:demo-admin --password MercaPOS2026 \
  --puesto puesto:demo --operaciones 200 --concurrencia 30
```

Genera `carga_*.csv`, `carga_*.log`, `metricas_antes_*.txt` y `metricas_despues_*.txt`.
La cuenta debe ser administradora para leer `/metrics`.

Para documentar CPU/RAM durante la carga, ejecute el muestreo en una terminal y la
prueba de carga en otra. El informe de ejemplo está en
`resultados_metrologia/Informe_Metrologia_MercaPOS.pdf`.
