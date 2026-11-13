# TinyTop v0.0.4 - Monitor ligero en Rust

[![CI](https://github.com/dev1lsconf/tinytop/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/dev1lsconf/tinytop/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/dev1lsconf/tinytop?display_name=tag)](https://github.com/dev1lsconf/tinytop/releases)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![version](https://img.shields.io/badge/version-0.0.4-blue.svg)](Cargo.toml)
[![rust](https://img.shields.io/badge/Rust-1.88%2B%20(edition%202024)-orange.svg)](https://www.rust-lang.org)
[![platform](https://img.shields.io/badge/platform-linux%20%7C%20BSD-lightgrey.svg)](https://www.kernel.org)

![TinyTop Dashboard](docs/dashboard.png?v=0.0.4-2)

**TinyTop** es un monitor autohospedado y ligero para servidores. Recolecta métricas del sistema y sondea targets HTTP configurables, persiste histórico en SQLite y expone un dashboard web con gráficos en tiempo real, además de una **TUI** interactiva en terminal. Funciona como **un solo binario**, sin Prometheus ni servicios externos.

## Características

- **Sistema**: CPU (%), memoria y disco (filesystems reales, deduplicados), red RX/TX y tasas; métricas extendidas best-effort (load, swap, uptime, I/O de disco, temperatura, top procesos). Colector con refrescos incrementales.
- **Targets (checks)**: sondeo concurrente con `check_type` `http` (integraciones compatibles), `tcp`, `ping` (best-effort) y `tls` (handshake y días de certificado, best-effort). Estado, latencia, detalle y resultados estables.
- **Persistencia**: SQLite (WAL, pragmas) con migraciones, histórico por rango, rollups/downsampling para rangos largos, alertas/eventos y pruning automático.
- **Alertas y notificaciones**: umbrales configurables (sistema/targets/latencia/cert), historial de incidentes y notificaciones opcionales ante transiciones (webhook, Telegram, ntfy, best-effort).
- **API**: `/api/health`, `/api/metrics/latest`, `/api/metrics/history`, `/api/config` (redactada), `/api/alerts`, `/api/events`, `/api/export` (JSON/CSV) y `/metrics` (Prometheus).
- **Dashboard**: HTML/CSS/JS embebido + Chart.js local (sin CDN), métricas extendidas e histórico, alertas/eventos, exportación y render seguro.
- **TUI**: `--tui` con vistas sistema/targets/alertas/eventos, métricas extendidas, sparklines y selección de rango/target (`Tab/1-4`, `+/-`, `↑/↓`, `r`, `q`).
- **Operación**: apagado ordenado, logging, config por archivo/env/CLI, `--config`/`--check-config`, recarga por `SIGHUP` (Unix), auth opcional (token/Basic) y cabeceras de seguridad.
- **Build/Release**: binario optimizado, imagen multi-stage no-root; releases con Linux (x86_64/aarch64/musl) y BSD, e imagen Docker multi-arch (amd64/arm64).

## Configuración

`config.toml` (ejemplo):

```toml
bind = "0.0.0.0"
port = 9090
db_path = "tinytop.db"
interval_seconds = 30
retention_hours = 168
request_timeout_secs = 5
max_history_hours = 168

[[targets]]
name = "localhost"
url = "http://localhost:8080/actuator/health"
kind = "spring-boot"

[[targets]]
name = "api"
url = "http://localhost:3000/api/health"
kind = "generic"
```

### Variables de entorno

| Variable | Descripción | Default |
|---|---|---|
| `TINYTOP_CONFIG` | Ruta al archivo de configuración | `config.toml` |
| `TINYTOP_BIND` | Dirección de escucha HTTP | `0.0.0.0` |
| `TINYTOP_PORT` | Puerto HTTP | `9090` |
| `TINYTOP_DB_PATH` | Ruta de la base SQLite | `tinytop.db` |
| `TINYTOP_INTERVAL` | Intervalo de recolección (segundos) | `30` |
| `TINYTOP_RETENTION_HOURS` | Retención de histórico (horas) | `168` |
| `TINYTOP_MAX_HISTORY_HOURS` | Máximo de horas consultables en historial (acotado por retención) | `168` |
| `TINYTOP_REQUEST_TIMEOUT_SECS` | Timeout de sondeo de targets (segundos) | `5` |

## API

- `GET /` — dashboard web
- `GET /api/health` — estado/readiness (`db.ok`, `runtime`, última colección, errores, versión)
- `GET /api/metrics/latest` — `{ system, targets }` (métricas extendidas y estado de checks/TLS)
- `GET /api/metrics/history?range=24&target=system|<name>` — histórico (downsampling en rangos largos)
- `GET /api/config` — configuración efectiva (valores sensibles redactados)
- `GET /api/alerts` — incidentes/alertas (`?status=&limit=`)
- `GET /api/events` — eventos recientes (`?limit=`)
- `GET /api/export?target=system|<name>&format=json|csv&range=24` — exportación
- `GET /metrics` — formato Prometheus

## Ejecución local

```bash
cargo build --release --locked
./target/release/tinytop
# Dashboard web: http://localhost:9090

# Modo TUI (terminal interactiva)
./target/release/tinytop --tui

# Ayuda y versión
./target/release/tinytop --help
./target/release/tinytop --version
```

## Docker

```bash
docker compose up -d --build
# http://localhost:9090
```

- DB persistida en volumen (`/app/data/tinytop.db` por defecto en la imagen/compose).
- Healthcheck: `curl -fsS http://localhost:9090/api/health`.
- Ejecuta como usuario no-root.
- En contenedor, las métricas de disco priorizan filesystems reales del host/volumen (se excluyen pseudo-FS como `overlay`/`tmpfs`).
- Para abrir la TUI en un contenedor en ejecución (requiere TTY): `docker exec -it tinytop tinytop --tui` (o el nombre de tu contenedor). Use la misma config/DB montada.

## Compatibilidad

- Soporte de plataforma: **Linux** y **sistemas *BSD** (FreeBSD, OpenBSD, NetBSD, DragonFlyBSD) vía `sysinfo`.
- Artefactos oficiales de release: Linux x86_64 (tarball) e imagen de contenedor `ghcr.io/dev1lsconf/tinytop`, además de tarballs para **FreeBSD**, **OpenBSD** y **NetBSD** (x86_64) generados por el workflow de release.
- En otras plataformas soportadas por las dependencias se puede compilar desde código (`cargo build --release --locked`).

## Pruebas y calidad

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
```

Pruebas E2E (Playwright, requiere servidor corriendo y accesible):

```bash
npm ci
npx playwright install --with-deps chromium
BASE_URL=http://localhost:9090 npx playwright test
```

El workflow de CI ejecuta fmt, clippy, tests, build y E2E en Ubuntu.

## Seguridad y notas

- Sin autenticación incorporada: expón el servicio detrás de reverse proxy con TLS/auth si es accesible públicamente.
- Respuestas con cabeceras de seguridad; la UI embebida usa scripts/estilos inline y Chart.js local (CSP acorde). Para endurecer más, sirve detrás de proxy con políticas adicionales según tu entorno.
- Las URLs de targets se leen de configuración local/controlada por el operador.
- `Cargo.lock` incluido para builds reproducibles.

## Changelog

Ver [CHANGELOG.md](CHANGELOG.md).

## Release y paquete

- Releases y artefactos (`.tar.gz` + `.sha256`): [GitHub Releases](https://github.com/dev1lsconf/tinytop/releases).
- Paquete de contenedor: `ghcr.io/dev1lsconf/tinytop` (publicado por el workflow de release en tags `v*`).
- El workflow `.github/workflows/release.yml` construye y publica artefactos para Linux x86_64 y *BSD (FreeBSD, OpenBSD, NetBSD), crea/actualiza la release y publica la imagen en GHCR; soporta `workflow_dispatch` para republicar desde un tag.

## Licencia

MIT - Copyright (c) 2026 TinyTop Contributors

## Agradecimientos

- [Rust](https://www.rust-lang.org), [Tokio](https://tokio.rs), [Axum](https://github.com/tokio-rs/axum), [SQLx](https://github.com/launchbadge/sqlx), [sysinfo](https://github.com/GuillaumeGomez/sysinfo), [Chart.js](https://www.chartjs.org/)
