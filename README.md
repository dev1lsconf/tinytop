# TinyTop v0.0.2 - Monitor ligero de VPS en Rust

[![CI](https://github.com/dev1lsconf/tinytop/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/dev1lsconf/tinytop/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![version](https://img.shields.io/badge/version-0.0.2-blue.svg)](Cargo.toml)
[![rust](https://img.shields.io/badge/Rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![platform](https://img.shields.io/badge/platform-linux-lightgrey.svg)](https://www.kernel.org)

![TinyTop Dashboard](docs/dashboard.png)

**TinyTop** es un monitor autohospedado y ligero para servidores/VPS. Recolecta métricas del sistema y sondea targets HTTP configurables, persiste histórico en SQLite y expone un dashboard web con gráficos en tiempo real. Funciona como **un solo binario**, sin Prometheus ni servicios externos.

## Características

- **Sistema**: CPU (%), memoria usada/total (%), disco usado/total/disponible (%), red acumulada y **tasas RX/TX por segundo** (con manejo de reinicio/overflow de contadores). El disco contabiliza filesystems reales y deduplica por dispositivo para evitar dobles conteos (bind mounts/subvolúmenes, p. ej. Btrfs/Docker).
- **Targets HTTP**: sondeo concurrente con cliente HTTP reutilizado, `kind` (`generic`, `spring-boot`/`spring`/`actuator`, `quarkus`, `micronaut`/`micrometer`), latencia, estado normalizado, extracción heurística de `http_requests` (JSON/Actuator/texto de métricas) y detalle de error acotado. Resultados en orden estable.
- **Persistencia y retención**: SQLite (WAL, `foreign_keys`, `synchronous=NORMAL`, `busy_timeout`, `temp_store=MEMORY`, `PRAGMA optimize`) con migraciones integradas y tablas `system_metrics`/`target_metrics`, histórico por rango, pruning automático por retención.
- **API**: `/api/health` (readiness/db/runtime), `/api/metrics/latest`, `/api/metrics/history`, `/api/config` con errores JSON y códigos HTTP consistentes (404/400/5xx).
- **Dashboard**: HTML/CSS/JS embebido + Chart.js servido localmente (`/static/vendor/chart.umd.min.js`, sin CDN), controles de rango y auto-refresh, estados vacíos/carga/error, render seguro de targets (DOM + escape), tabla y tarjetas accesibles.
- **Operación**: apagado ordenado (SIGINT/SIGTERM), logging con `tracing`, configuración por archivo + overrides de entorno, validación estricta (bind/puerto/rangos/nombres únicos/URLs/kinds), cabeceras de seguridad HTTP (`nosniff`, `no-referrer`, `DENY`, `Permissions-Policy`, CSP compatible con la UI embebida).
- **Build/Docker**: binario `release` optimizado (`lto`, `strip`, `panic=abort`), imagen multi-stage, usuario no-root, volumen de datos, `HEALTHCHECK`, `Cargo.lock` para builds reproducibles.

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
- `GET /api/metrics/latest` — `{ system: {...} | null, targets: [...] }`
- `GET /api/metrics/history?range=24&target=system` — `{ target, range_hours, points: [...] }` (sistema; `range` se acota a `max_history_hours`/retención)
- `GET /api/metrics/history?range=24&target=<name>` — histórico del target (404 si no existe)
- `GET /api/config` — configuración efectiva (`bind`, `port`, `db_path`, intervalos, retención, `max_history_hours`, timeout, targets)

## Ejecución local

```bash
cargo build --release --locked
./target/release/tinytop
# http://localhost:9090
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

## Licencia

MIT - Copyright (c) 2026 TinyTop Contributors

## Agradecimientos

- [Rust](https://www.rust-lang.org), [Tokio](https://tokio.rs), [Axum](https://github.com/tokio-rs/axum), [SQLx](https://github.com/launchbadge/sqlx), [sysinfo](https://github.com/GuillaumeGomez/sysinfo), [Chart.js](https://www.chartjs.org/)