#!/usr/bin/env python3
"""Genera carga concurrente de compras y documenta latencias y métricas.

Mide cada transacción HTTP (POST /operaciones), calcula p50/p95/p99 y
throughput, y guarda las exposiciones Prometheus antes/después de la carga.
Depende solo de la biblioteca estándar.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
import sys
import time
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen


def peticion(api: str, method: str, path: str, body=None, token=None, timeout=60):
    headers = {"Accept": "application/json"}
    if body is not None:
        headers["Content-Type"] = "application/json"
    if token:
        headers["Authorization"] = f"Bearer {token}"
    data = json.dumps(body).encode() if body is not None else None
    request = Request(api + path, data=data, headers=headers, method=method)
    inicio = time.perf_counter()
    try:
        with urlopen(request, timeout=timeout) as response:
            crudo = response.read()
            estado = response.status
    except HTTPError as error:
        crudo = error.read()
        estado = error.code
    except URLError as error:
        return 0, None, (time.perf_counter() - inicio) * 1000, str(error)
    duracion = (time.perf_counter() - inicio) * 1000
    try:
        datos = json.loads(crudo) if crudo else None
    except ValueError:
        datos = crudo.decode(errors="replace")
    return estado, datos, duracion, None


def percentil(valores: list[float], p: float) -> float:
    if not valores:
        return 0.0
    ordenados = sorted(valores)
    indice = max(math.ceil(p / 100.0 * len(ordenados)) - 1, 0)
    return ordenados[indice]


def extraer_metricas(texto: str) -> dict[str, float]:
    claves = {
        "requests": 'mercapos_http_requests_total ',
        "errores_5xx": 'mercapos_http_errors_total ',
        "hilos": 'mercapos_process_threads ',
        "rss_bytes": 'mercapos_process_resident_memory_bytes ',
        "ops_count": 'mercapos_http_request_duration_milliseconds_count{method="POST",route="/operaciones"} ',
        "ops_sum_ms": 'mercapos_http_request_duration_milliseconds_sum{method="POST",route="/operaciones"} ',
        "ops_inf": 'mercapos_http_request_duration_milliseconds_bucket{method="POST",route="/operaciones",le="+Inf"} ',
    }
    resultado = {}
    for nombre, prefijo in claves.items():
        valor = next(
            (float(linea[len(prefijo):].strip()) for linea in texto.splitlines() if linea.startswith(prefijo)),
            None,
        )
        if valor is None:
            valor = -1.0 if nombre in ("hilos", "rss_bytes") else 0.0
        resultado[nombre] = valor
    return resultado


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api", default="http://127.0.0.1:3000")
    parser.add_argument("--usuario", default="usuario:demo-admin")
    parser.add_argument("--password", default="MercaPOS2026")
    parser.add_argument("--puesto", default="puesto:demo")
    parser.add_argument("--operaciones", type=int, default=100)
    parser.add_argument("--concurrencia", type=int, default=20)
    parser.add_argument("--salida", default="resultados_metrologia", help="Carpeta de resultados")
    args = parser.parse_args()
    api = args.api.rstrip("/")

    estado, login, _, error = peticion(
        api, "POST", "/auth/login", {"usuario_id": args.usuario, "password": args.password}
    )
    if estado != 200 or not isinstance(login, dict) or "token" not in login:
        print(f"No se pudo iniciar sesión: HTTP {estado} {login or error}", file=sys.stderr)
        return 1
    token = login["token"]

    query = urlencode({"puesto_id": args.puesto, "activo": "true"})
    estado, productos, _, error = peticion(api, "GET", f"/productos?{query}", token=token)
    if estado != 200 or not productos:
        print(f"No hay productos para la prueba: HTTP {estado} {productos or error}", file=sys.stderr)
        return 1
    producto_id = productos[0]["id"]

    antes_texto = None
    estado_m, antes_texto, _, _ = peticion(api, "GET", "/metrics", token=token)
    if estado_m != 200 or not isinstance(antes_texto, str):
        print("La cuenta no puede leer /metrics (se requiere administrador).", file=sys.stderr)
        return 1

    salida = Path(args.salida)
    salida.mkdir(parents=True, exist_ok=True)
    marca = datetime.now().strftime("%Y%m%d_%H%M%S")
    csv_path = salida / f"carga_{marca}.csv"
    log_path = salida / f"carga_{marca}.log"
    metricas_antes_path = salida / f"metricas_antes_{marca}.txt"
    metricas_despues_path = salida / f"metricas_despues_{marca}.txt"
    metricas_antes_path.write_text(antes_texto)

    fecha = datetime.now().astimezone().isoformat(timespec="seconds")
    dispositivo = f"carga-{uuid.uuid4().hex[:8]}"
    corrida = uuid.uuid4().hex[:10]

    def transaccion(indice: int):
        identificador = f"operacion:carga:{corrida}:{indice:04d}"
        cuerpo = {
            "id": identificador,
            "tipo_operacion": "compra",
            "puesto_id": args.puesto,
            "usuario_id": args.usuario,
            "dispositivo_id": dispositivo,
            "fecha_local": fecha,
            "renglones": [
                {"producto_id": producto_id, "cantidad": 1, "costo_unitario": 1.0}
            ],
            "total": 1.0,
        }
        estado, datos, duracion, error = peticion(api, "POST", "/operaciones", cuerpo, token=token)
        return indice, estado, datos, duracion, error

    inicio = time.perf_counter()
    filas = []
    with ThreadPoolExecutor(max_workers=args.concurrencia) as pool:
        futuros = [pool.submit(transaccion, i) for i in range(args.operaciones)]
        for futuro in as_completed(futuros):
            indice, estado, datos, duracion, error = futuro.result()
            aceptada = estado in (200, 201)
            filas.append(
                {
                    "operacion": indice,
                    "estado_http": estado,
                    "aceptada": "si" if aceptada else "no",
                    "duracion_ms": round(duracion, 2),
                    "error": error or ("" if aceptada else str(datos)[:160]),
                }
            )
    total_segundos = time.perf_counter() - inicio
    filas.sort(key=lambda fila: fila["operacion"])

    with csv_path.open("w", newline="") as archivo:
        writer = csv.DictWriter(archivo, fieldnames=list(filas[0].keys()))
        writer.writeheader()
        writer.writerows(filas)

    _, despues_texto, _, _ = peticion(api, "GET", "/metrics", token=token)
    if not isinstance(despues_texto, str):
        despues_texto = ""
    metricas_despues_path.write_text(despues_texto)
    antes = extraer_metricas(antes_texto)
    despues = extraer_metricas(despues_texto)

    duraciones = [fila["duracion_ms"] for fila in filas]
    aceptadas = sum(1 for fila in filas if fila["aceptada"] == "si")
    errores = len(filas) - aceptadas
    throughput = args.operaciones / total_segundos if total_segundos else 0.0
    ops_delta = despues["ops_count"] - antes["ops_count"] if antes["ops_count"] >= 0 else -1
    suma_delta = despues["ops_sum_ms"] - antes["ops_sum_ms"] if antes["ops_sum_ms"] >= 0 else -1
    promedio_servidor = suma_delta / ops_delta if ops_delta > 0 else -1

    lineas = [
        "MercaPOS - Prueba de carga concurrente",
        f"Inicio: {datetime.now().astimezone().isoformat(timespec='seconds')}",
        f"API: {api} | Usuario: {args.usuario} | Producto: {producto_id}",
        f"Operaciones: {args.operaciones} | Concurrencia: {args.concurrencia}",
        "",
        "Resultados del cliente:",
        f"  Duración total: {total_segundos:.3f} s",
        f"  Throughput: {throughput:.2f} operaciones/s",
        f"  Aceptadas: {aceptadas} | Errores: {errores}",
        f"  Latencia mínima: {min(duraciones):.2f} ms",
        f"  Latencia media: {statistics.fmean(duraciones):.2f} ms",
        f"  Latencia p50: {percentil(duraciones, 50):.2f} ms",
        f"  Latencia p95: {percentil(duraciones, 95):.2f} ms",
        f"  Latencia p99: {percentil(duraciones, 99):.2f} ms",
        f"  Latencia máxima: {max(duraciones):.2f} ms",
        "",
        "Telemetría del servidor (delta durante la carga):",
        f"  Solicitudes POST /operaciones: {ops_delta:.0f}",
        f"  Tiempo acumulado de procesamiento: {suma_delta:.0f} ms",
        f"  Tiempo medio por transacción: {promedio_servidor:.2f} ms" if promedio_servidor >= 0 else "  Tiempo medio por transacción: no disponible",
        f"  Hilos del proceso (después): {despues['hilos']:.0f}" if despues['hilos'] >= 0 else "  Hilos del proceso: no disponible",
        f"  RSS del proceso (después): {despues['rss_bytes'] / (1024 * 1024):.2f} MiB" if despues['rss_bytes'] >= 0 else "  RSS del proceso: no disponible",
        f"  Errores 5xx acumulados: {despues['errores_5xx']:.0f}",
        "",
        "Archivos:",
        f"  CSV: {csv_path}",
        f"  LOG: {log_path}",
        f"  Métricas antes: {metricas_antes_path}",
        f"  Métricas después: {metricas_despues_path}",
        "Nota: la latencia del cliente incluye red local, HTTP y persistencia en CouchDB.",
    ]
    log_path.write_text("\n".join(lineas) + "\n")
    print("\n".join(lineas))
    return 0 if errores == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
