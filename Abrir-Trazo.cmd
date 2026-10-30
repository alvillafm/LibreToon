@echo off
cd /d "%~dp0"
if exist "target\release\libretoon.exe" (
  start "" "target\release\libretoon.exe"
) else (
  cargo run --release --locked
  if errorlevel 1 pause
)

