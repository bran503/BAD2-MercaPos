#!/usr/bin/env python3
"""Crea la cuenta demo y un catálogo inicial si la base está vacía."""

import json
import os
import sys
import time
from datetime import datetime
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

API = os.environ.get("API_URL", "http://api:3000").rstrip("/")
BOOTSTRAP_TOKEN = os.environ.get("MERCAPOS_BOOTSTRAP_TOKEN", "")
USUARIO = os.environ.get("DEMO_USER", "usuario:demo-admin")
PASSWORD = os.environ.get("DEMO_PASSWORD", "MercaPOS2026")
PUESTO = os.environ.get("DEMO_PUESTO", "puesto:demo")
PRODUCTOS = [
    ("producto:demo-tomate", "Tomate", "verduras", 1.25, "libra", 50),
    ("producto:demo-cebolla", "Cebolla", "verduras", 0.90, "libra", 30),
    ("producto:demo-papa", "Papa", "verduras", 0.75, "libra", 40),
    ("producto:demo-zanahoria", "Zanahoria", "verduras", 0.80, "libra", 25),
    ("producto:demo-lechuga", "Lechuga", "verduras", 0.60, "unidad", 20),
    ("producto:demo-manzana", "Manzana", "frutas", 1.10, "libra", 35),
]


def peticion(method, path, body=None, token=None):
    headers = {"Accept": "application/json"}
    if body is not None:
        headers["Content-Type"] = "application/json"
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if BOOTSTRAP_TOKEN:
        headers["X-Bootstrap-Token"] = BOOTSTRAP_TOKEN
    data = json.dumps(body).encode() if body is not None else None
    request = Request(API + path, data=data, headers=headers, method=method)
    try:
        with urlopen(request, timeout=15) as response:
            raw = response.read()
            return response.status, json.loads(raw) if raw else None
    except HTTPError as error:
        raw = error.read()
        try:
            detail = json.loads(raw) if raw else None
        except ValueError:
            detail = None
        return error.code, detail


def esperar_api():
    for _ in range(90):
        try:
            with urlopen(API + "/health", timeout=5) as response:
                if response.status == 200:
                    return
        except (URLError, OSError):
            time.sleep(2)
    raise RuntimeError("La API no respondió a tiempo")


def login():
    return peticion("POST", "/auth/login", {"usuario_id": USUARIO, "password": PASSWORD})


def main():
    esperar_api()
    estado, cuerpo = login()
    if estado == 200:
        print("Datos demo ya inicializados; no se crean duplicados.")
        print(f"USUARIO={USUARIO}")
        print(f"CONTRASEÑA={PASSWORD}")
        return 0

    estado, creado = peticion(
        "POST",
        "/auth/bootstrap",
        {
            "puesto_id": PUESTO,
            "puesto_nombre": "Puesto de demostración",
            "mercado": "Mercado de prueba",
            "ubicacion": "El Salvador",
            "usuario_id": USUARIO,
            "usuario_nombre": "Administrador demo",
            "password": PASSWORD,
        },
    )
    if estado in (403, 409):
        estado, cuerpo = login()
        if estado != 200:
            raise RuntimeError(f"No se pudo iniciar sesión demo: HTTP {estado} {cuerpo}")
        print("Base ya inicializada; no se crean duplicados.")
        print(f"USUARIO={USUARIO}")
        print(f"CONTRASEÑA={PASSWORD}")
        return 0
    if estado not in (200, 201) or not creado or not creado.get("token"):
        raise RuntimeError(f"Bootstrap falló: HTTP {estado} {creado}")

    token = creado["token"]
    fecha = datetime.now().astimezone().isoformat(timespec="seconds")
    for producto_id, nombre, categoria, precio, unidad, cantidad in PRODUCTOS:
        peticion(
            "POST",
            "/productos",
            {
                "id": producto_id,
                "puesto_id": PUESTO,
                "nombre": nombre,
                "categoria": categoria,
                "precio_unitario": precio,
                "unidad": unidad,
                "actualizado_en": fecha,
                "actualizado_por": USUARIO,
            },
            token=token,
        )
        peticion(
            "POST",
            "/movimientos",
            {
                "id": f"mov:demo-{nombre.lower()}-inicial",
                "producto_id": producto_id,
                "puesto_id": PUESTO,
                "cantidad": cantidad,
                "motivo": "inventario_inicial",
                "dispositivo_id": "demo-web",
                "usuario_id": USUARIO,
                "fecha_local": fecha,
            },
            token=token,
        )

    query = urlencode({"puesto_id": PUESTO, "activo": "true"})
    estado, catalogo = peticion("GET", f"/productos?{query}", token=token)
    if estado != 200:
        raise RuntimeError(f"No se pudo verificar el catálogo: HTTP {estado} {catalogo}")
    print(f"LISTO: {len(catalogo)} productos demo creados.")
    print(f"USUARIO={USUARIO}")
    print(f"CONTRASEÑA={PASSWORD}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:
        print(f"ERROR: {error}", file=sys.stderr)
        sys.exit(1)
