# Changelog

Todos los cambios notables de este proyecto se documentan en este archivo.

El formato se basa en [Keep a Changelog](https://keepachangelog.com/es/1.1.0/).

## [0.1.0] - 2026-11-13

> Bump MENOR (SemVer): nuevas funcionalidades compatibles con versiones anteriores.

### Añadido
- Modo **TUI** interactivo en terminal (`--tui`) con vistas de sistema/targets/alertas/eventos, métricas extendidas, sparklines de histórico y selección de rango/target.
- Flags de CLI `--help`/`-h`, `--version`/`-V`, `--config <PATH>`, `--check-config`.
- Métricas de sistema extendidas (best-effort): load average, swap, uptime, I/O de disco, temperatura y top de procesos.
- Checks de targets adicionales: `tcp`, `ping` y `tls` (con latencia y días de certificado, best-effort).
- Alertas e incidentes configurables (sistema/targets/latencia/cert) con historial y eventos, y notificaciones opcionales (webhook, Telegram, ntfy).
- API ampliada: `/api/alerts`, `/api/events`, `/api/export` (JSON/CSV), `/metrics` (Prometheus) y campos extendidos en métricas/config (redactada).
- Autenticación opcional (Bearer/`X-API-Key`/`?token=` o Basic) y recarga de configuración por `SIGHUP` (Unix).
- Rollups/downsampling de métricas e índices/migración `20241202000002_extended_metrics`.
- Dashboard web con métricas extendidas, alertas/eventos y exportación.
- Pruebas de integración de API y de componentes (alertas/rollups/auth/config), refactor a biblioteca, `CONTRIBUTING.md` y documentación ampliada.
- Release multi-arch y empaquetado nativo: binarios Linux (x86_64, aarch64, musl) y *BSD, imagen Docker multi-arch (amd64/arm64) y artefactos .deb/.rpm.

## [0.0.5] - 2026-11-13

(versión intermedia consolidada en 0.1.0)

### Añadido
- Modo **TUI** interactivo en terminal (`--tui`) con vistas de sistema/targets/alertas/eventos, métricas extendidas, sparklines de histórico y selección de rango/target.
- Flags de CLI `--help`/`-h`, `--version`/`-V`, `--config <PATH>`, `--check-config`.
- Métricas de sistema extendidas (best-effort): load average, swap, uptime, I/O de disco, temperatura y top de procesos.
- Checks de targets adicionales: `tcp`, `ping` y `tls` (con latencia y días de certificado, best-effort).
- Alertas e incidentes configurables (sistema/targets/latencia/cert) con historial y eventos, y notificaciones opcionales (webhook, Telegram, ntfy).
- API ampliada: `/api/alerts`, `/api/events`, `/api/export` (JSON/CSV), `/metrics` (Prometheus) y campos extendidos en métricas/config (redactada).
- Autenticación opcional (Bearer/`X-API-Key`/`?token=` o Basic) y recarga de configuración por `SIGHUP` (Unix).
- Rollups/downsampling de métricas e índices/migración `20241202000002_extended_metrics`.
- Dashboard web con métricas extendidas, alertas/eventos y exportación; TUI enriquecida.
- Pruebas de integración de API, refactor a biblioteca, `CONTRIBUTING.md` y documentación ampliada.
- Release multi-arch: binarios Linux (x86_64, aarch64, musl) y *BSD, e imagen Docker multi-arch (amd64/arm64).

## [0.0.4] - 2026-11-13

### Añadido
- Soporte de plataforma para **sistemas *BSD** (FreeBSD, OpenBSD, NetBSD, DragonFlyBSD) vía `sysinfo`, manteniendo Linux.
- Artefactos de release para *BSD (tarballs `freebsd`/`openbsd`/`netbsd` x86_64) generados y publicados por el workflow de release.
- Sección de compatibilidad en el README (artefactos oficiales y compilación desde código en plataformas soportadas).

### Cambiado
- Colector de disco: excluye pseudo-filesystems adicionales presentes en *BSD/Linux (`procfs`, `devfs`, `fdescfs`, `linprocfs`, `linsysfs`, `nullfs`) para contabilizar solo almacenamiento real.
- Versión del proyecto actualizada a `0.0.4`.

### Corregido
- Pruebas de base de datos: uso de `std::env::temp_dir()` en lugar de `/tmp` fijo para mayor portabilidad.

## [0.0.3] - 2026-11-13

### Añadido
- `CHANGELOG.md` y archivo `LICENSE` (MIT).
- Workflow de release con `workflow_dispatch` para publicar desde un tag existente.
- Documentación de releases y paquete de contenedor (GHCR) en el README.

### Cambiado
- Versión del proyecto actualizada a `0.0.3`.
- Colector de sistema: reutiliza estado de `sysinfo` (`System`/`Disks`/`Networks`) entre ciclos con refrescos incrementales y línea base de CPU para menor sobrecarga.
- Refactors de estilo compatibles con Rust 2024 (correcciones `clippy::collapsible_if`).

## [0.0.2] - 2026-11-13

### Añadido
- Configuración `bind` y `max_history_hours` con validación reforzada.
- Cabeceras de seguridad HTTP y migraciones SQLite integradas con pragmas de bajo coste.
- Workflow automático de release/publicación de paquete y badge de CI.

### Cambiado
- Cálculo de disco sobre filesystems reales con deduplicación por dispositivo.
- Sondeo de targets HTTP concurrente con cliente reutilizado y orden estable.
- Build `release` optimizado y actualización a Rust edition 2024.

### Corregido
- Conteos de disco inflados por pseudo-filesystems/bind mounts.

## [0.0.1] - 2026-11-12

### Añadido
- Versión inicial: métricas de sistema y targets HTTP, SQLite, API y dashboard web en un solo binario.
