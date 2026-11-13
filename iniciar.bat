@echo off
cd /d "%~dp0"
if not exist .env copy .env.example .env
docker compose up --build -d
echo MercaPOS iniciando. Web: http://localhost:8080
echo Credenciales: usuario:demo-admin / MercaPOS2026
pause
