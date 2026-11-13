#!/usr/bin/env python3
"""Muestrea CPU, RAM e hilos de la API Rust y de CouchDB.

Genera un CSV por muestra y un LOG con el resumen estadístico, pensados para
documentar la metrología del proyecto. Solo usa la biblioteca estándar y
`docker stats` cuando se mide un contenedor.
"""

from __future__ import annotations

import argparse
import csv
import os
import platform
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path


def ahora_iso() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def leer_estado(pid: int) -> tuple[float, float, int]:
    """Devuelve (rss_mib, cpu_pct, hilos) del proceso, o valores -1."""
    try:
        status = Path(f"/proc/{pid}/status").read_text()
    except OSError:
        return -1.0, -1.0, -1
    rss_kib = 0.0
    threads = 0
    for line in status.splitlines():
        if line.startswith("VmRSS:"):
            rss_kib = float(line.split()[1])
        elif line.startswith("Threads:"):
            threads = int(line.split()[1])
    try:
        stat = Path(f"/proc/{pid}/stat").read_text()
        resto = stat[stat.rindex(")") + 2 :].split()
        ticks = int(resto[11]) + int(resto[12])
    except (OSError, ValueError, IndexError):
        return rss_kib / 1024.0, -1.0, threads
    return rss_kib / 1024.0, ticks, threads


def ticks_por_segundo() -> int:
    try:
        return int(os.sysconf("SC_CLK_TCK"))
    except (ValueError, OSError):
        return 100


def auto_pid() -> int | None:
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            cmdline = (entry / "cmdline").read_bytes().replace(b"\0", b" ").decode(errors="replace")
        except OSError:
            continue
        if "/backend" in cmdline and "python" not in cmdline and "muestrear" not in cmdline:
            return int(entry.name)
    return None


def auto_contenedor() -> str | None:
    try:
        salida = subprocess.run(
            ["docker", "ps", "--format", "{{.Names}}"],
            check=True, capture_output=True, text=True, timeout=10,
        ).stdout
    except (OSError, subprocess.SubprocessError):
        return None
    for nombre in salida.splitlines():
        if "couchdb" in nombre.lower():
            return nombre
    return None


def docker_stats(nombre: str) -> tuple[float, float]:
    """Devuelve (cpu_pct, memoria_mib) del contenedor."""
    try:
        salida = subprocess.run(
            ["docker", "stats", "--no-stream", "--format", "{{.CPUPerc}}|{{.MemUsage}}", nombre],
            check=True, capture_output=True, text=True, timeout=20,
        ).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return -1.0, -1.0
    if not salida:
        return -1.0, -1.0
    cpu_texto, memoria_texto = salida.split("|", 1)
    cpu = float(cpu_texto.replace("%", "").strip()) if cpu_texto.strip() else -1.0
    uso = memoria_texto.split("/")[0].strip()
    unidades = {"B": 1 / (1024 * 1024), "KiB": 1 / 1024, "MiB": 1, "GiB": 1024, "TiB": 1024 * 1024}
    numero = "".join(ch for ch in uso if ch.isdigit() or ch == ".")
    sufijo = uso[len(numero):].strip()
    try:
        memoria = float(numero) * unidades.get(sufijo, 1.0)
    except ValueError:
        memoria = -1.0
    return cpu, memoria


def promedio(valores: list[float]) -> float:
    validos = [v for v in valores if v >= 0]
    return statistics.fmean(validos) if validos else -1.0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api-pid", type=int, help="PID del proceso Rust (se autodetecta)")
    parser.add_argument("--api-container", help="Nombre del contenedor de la API (alternativa al PID)")
    parser.add_argument("--couchdb-container", help="Nombre del contenedor CouchDB (se autodetecta)")
    parser.add_argument("--duracion", type=float, default=30, help="Segundos de muestreo")
    parser.add_argument("--intervalo", type=float, default=1, help="Segundos entre muestras")
    parser.add_argument("--salida", default="resultados_metrologia", help="Carpeta de resultados")
    args = parser.parse_args()

    pid = args.api_pid or (None if args.api_container else auto_pid())
    contenedor_couch = args.couchdb_container or auto_contenedor()
    if pid is None and not args.api_container:
        print("No se encontró el proceso de la API; use --api-pid o --api-container.", file=sys.stderr)
        return 1
    if contenedor_couch is None:
        print("No se encontró el contenedor de CouchDB; use --couchdb-container.", file=sys.stderr)
        return 1

    salida = Path(args.salida)
    salida.mkdir(parents=True, exist_ok=True)
    marca = datetime.now().strftime("%Y%m%d_%H%M%S")
    csv_path = salida / f"recursos_{marca}.csv"
    log_path = salida / f"recursos_{marca}.log"

    inicio_iso = ahora_iso()
    filas = []
    inicio = time.monotonic()
    anterior_ticks = None
    anterior_tiempo = None
    print(f"Muestreando {args.duracion:.0f}s cada {args.intervalo:.1f}s...")
    while True:
        ahora = time.monotonic()
        transcurrido = ahora - inicio
        if transcurrido > args.duracion:
            break
        cpu_api = rss_api = -1.0
        hilos = -1
        if args.api_container:
            cpu_api, rss_api = docker_stats(args.api_container)
            try:
                salida_threads = subprocess.run(
                    ["docker", "exec", args.api_container, "sh", "-c", "grep Threads /proc/1/status"],
                    check=True, capture_output=True, text=True, timeout=10,
                ).stdout
                hilos = int(salida_threads.split()[1])
            except (OSError, subprocess.SubprocessError, ValueError, IndexError):
                hilos = -1
        else:
            rss_api, ticks, hilos = leer_estado(pid)
            if ticks >= 0 and anterior_ticks is not None and anterior_tiempo is not None:
                delta_tiempo = max(ahora - anterior_tiempo, 0.001)
                cpu_api = (ticks - anterior_ticks) / ticks_por_segundo() / delta_tiempo * 100.0
            if ticks >= 0:
                anterior_ticks = ticks
                anterior_tiempo = ahora
        cpu_couch, mem_couch = docker_stats(contenedor_couch)
        fila = {
            "muestra": len(filas) + 1,
            "hora": datetime.now(timezone.utc).isoformat(timespec="milliseconds"),
            "api_cpu_pct": round(cpu_api, 2),
            "api_rss_mib": round(rss_api, 2),
            "api_hilos": hilos,
            "couchdb_cpu_pct": round(cpu_couch, 2),
            "couchdb_mem_mib": round(mem_couch, 2),
        }
        filas.append(fila)
        print(
            f"[{fila['muestra']:02d}] API cpu={fila['api_cpu_pct']:.2f}% "
            f"rss={fila['api_rss_mib']:.2f} MiB hilos={hilos} | "
            f"CouchDB cpu={fila['couchdb_cpu_pct']:.2f}% mem={fila['couchdb_mem_mib']:.2f} MiB"
        )
        time.sleep(max(args.intervalo - (time.monotonic() - ahora), 0))

    with csv_path.open("w", newline="") as archivo:
        writer = csv.DictWriter(archivo, fieldnames=list(filas[0].keys()))
        writer.writeheader()
        writer.writerows(filas)

    def resumen(clave: str) -> str:
        valores = [f[clave] for f in filas if f[clave] >= 0]
        if not valores:
            return "sin datos"
        return (
            f"min={min(valores):.2f} media={estadistica(valores):.2f} "
            f"max={max(valores):.2f}"
        )

    def estadistica(valores: list[float]) -> float:
        return statistics.fmean(valores)

    lineas = [
        "MercaPOS - Muestreo de recursos",
        f"Inicio: {inicio_iso}",
        f"Fin: {ahora_iso()}",
        f"Equipo: {platform.node()} | {platform.platform()} | CPUs: {os.cpu_count()}",
        f"API: {'contenedor ' + args.api_container if args.api_container else 'PID ' + str(pid)}",
        f"CouchDB: contenedor {contenedor_couch}",
        f"Duración: {args.duracion:.0f}s | Intervalo: {args.intervalo:.1f}s | Muestras: {len(filas)}",
        "",
        "Resumen API Rust:",
        f"  CPU %:    {resumen('api_cpu_pct')}",
        f"  RSS MiB:  {resumen('api_rss_mib')}",
        f"  Hilos:    {resumen('api_hilos')}",
        "Resumen CouchDB:",
        f"  CPU %:    {resumen('couchdb_cpu_pct')}",
        f"  Mem MiB:  {resumen('couchdb_mem_mib')}",
        "",
        "Archivos:",
        f"  CSV: {csv_path}",
        f"  LOG: {log_path}",
        "Nota: CPU% puede superar 100 cuando el proceso usa varios hilos.",
    ]
    log_path.write_text("\n".join(lineas) + "\n")
    print("\n" + "\n".join(lineas))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
