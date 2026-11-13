#!/usr/bin/env sh
set -e
cd "$(dirname "$0")"
[ -f .env ] || cp .env.example .env
docker compose up --build -d
echo "MercaPOS iniciando. Web: http://localhost:8080"
echo "Credenciales: usuario:demo-admin / MercaPOS2026"
