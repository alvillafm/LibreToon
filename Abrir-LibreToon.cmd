@echo off
setlocal
cd /d "%~dp0"
set "CARGO_TARGET_DIR=%~dp0target"
echo Comprobando y compilando LibreToon...
cargo build --release --locked
if errorlevel 1 (
  echo No se pudo compilar LibreToon. Revisa el error anterior.
  pause
  exit /b 1
)
start "" "%~dp0target\release\libretoon.exe"
