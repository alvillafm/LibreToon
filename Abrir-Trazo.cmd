@echo off
cd /d "%~dp0"
if exist "target\debug\libretoon.exe" (
  start "" "target\debug\libretoon.exe"
) else (
  cargo run --locked
  if errorlevel 1 pause
)

