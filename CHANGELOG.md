# Changelog

Todos los cambios notables de este proyecto se documentan en este archivo.

El formato se basa en [Keep a Changelog](https://keepachangelog.com/es/1.1.0/).

## [Unreleased]

### Añadido
- Modo **TUI** interactivo en terminal (`--tui`) con vista en vivo de sistema, estado/runtime y targets (sobre la misma base de datos y colector en background). Atajos: `q`/`Esc`/`Ctrl+C` salir, `r` refrescar.
- Flags de CLI `--help`/`-h`, `--version`/`-V`, `--config <PATH>`, `--check-config` (validación de configuración).
- Métricas de sistema extendidas (best-effort según plataforma): load average, swap, uptime, I/O de disco, temperatura y top de procesos.
- Checks de targets adicionales: `tcp`, `ping` y `tls` (con latencia y días restantes de certificado en TLS/HTTPS best-effort).
- Alertas e incidentes configurables (CPU/mem/disco/temperatura, target caído/degradado, latencia, expiración de certificado) con historial en base de datos y eventos.
- Notificaciones opcionales ante transiciones de alerta: webhook genérico, Telegram y ntfy.
- API: `/api/alerts`, `/api/events`, `/api/export` (JSON/CSV), `/metrics` (Prometheus) y campos extendidos en métricas/latest/history/config.
- Autenticación opcional (token Bearer/`X-API-Key`/`?token=` o Basic Auth) con configuración sensible redactada en `/api/config`.
- Recarga de configuración en caliente por `SIGHUP` (Unix) y rollups/agregación periódica de métricas en SQLite (migración `20241202000002_extended_metrics`).

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
