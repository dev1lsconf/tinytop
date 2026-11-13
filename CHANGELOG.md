# Changelog

Todos los cambios notables de este proyecto se documentan en este archivo.

El formato se basa en [Keep a Changelog](https://keepachangelog.com/es/1.1.0/).

## [0.0.4] - 2026-11-13

### Añadido
- Soporte de plataforma para **sistemas *BSD** (FreeBSD, OpenBSD, NetBSD, DragonFlyBSD) vía `sysinfo`, manteniendo Linux.
- Sección de compatibilidad en el README (binarios oficiales Linux x86_64/contenedor; compilación desde código en plataformas soportadas).

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
