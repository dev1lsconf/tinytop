# TinyTop v0.1.0 - Monitor ligero en Rust

[![CI](https://github.com/dev1lsconf/tinytop/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/dev1lsconf/tinytop/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/dev1lsconf/tinytop?display_name=tag)](https://github.com/dev1lsconf/tinytop/releases)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![version](https://img.shields.io/badge/version-0.1.0-blue.svg)](Cargo.toml)
[![rust](https://img.shields.io/badge/Rust-1.88%2B%20(edition%202024)-orange.svg)](https://www.rust-lang.org)
[![platform](https://img.shields.io/badge/platform-linux%20%7C%20BSD-lightgrey.svg)](https://www.kernel.org)

![TinyTop Dashboard](docs/dashboard.png?v=0.1.0)

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

## Ejecución rápida

```bash
cargo build --release --locked
./target/release/tinytop --check-config
./target/release/tinytop            # Dashboard web: http://localhost:9090
./target/release/tinytop --tui      # TUI
```

### Docker / Docker Compose

```bash
docker compose up -d --build
# http://localhost:9090
```

- Monta `config.toml` como solo lectura y persiste la DB (`db_path`) en un volumen.
- La imagen expone `9090` y la TUI requiere TTY: `docker exec -it tinytop-dev tinytop --tui` (ajusta el nombre).

## Compatibilidad

- Soporte de plataforma: **Linux** y **sistemas *BSD** (FreeBSD, OpenBSD, NetBSD, DragonFlyBSD) vía `sysinfo`.
- Artefactos de release: Linux x86_64, aarch64 y musl (tarballs) e imagen de contenedor `ghcr.io/dev1lsconf/tinytop` multi-arch (amd64/arm64), además de tarballs para **FreeBSD**, **OpenBSD** y **NetBSD** (x86_64).
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

## Configuración avanzada (ejemplo)

```toml
# Recarga en caliente (Unix): kill -HUP <pid> | --config /ruta/config.toml

[auth]
token = "mi-token"           # o basic_user/basic_password
allow_health_unauth = true

[alerts]
cpu_percent_warn = 90.0
mem_percent_warn = 90.0
disk_percent_warn = 90.0
temp_celsius_warn = 85.0
target_latency_ms_warn = 1000.0
cert_expiry_days_warn = 14

[notifications]
webhook_url = "https://hooks.example/..."
# telegram_bot_token = "..." ; telegram_chat_id = "..."
# ntfy_url = "https://ntfy.sh" ; ntfy_topic = "tinytop"

[[targets]]
name = "web"
url = "https://example.com/health"
check_type = "http"

[[targets]]
name = "db"
check_type = "tcp"
host = "db.internal"
port = 5432

[[targets]]
name = "gateway"
check_type = "ping"
host = "10.0.0.1"

[[targets]]
name = "api-tls"
check_type = "tls"
host = "example.com"
port = 443
```

## Notas operativas y troubleshooting

- **Seguridad**: auth opcional (`[auth]`, token Bearer/`X-API-Key`/`?token=` o Basic); sin auth la API queda abierta — usar red privada/reverse proxy con TLS. Config sensible redactada en `/api/config`.
- **Checks best-effort**: `ping` y días de certificado TLS requieren `ping`/`openssl` en el entorno; pueden no estar disponibles en contenedores mínimos.
- **Sin datos / “starting”**: espera `interval_seconds`, revisa `/api/health` y logs; valida con `--check-config`.
- **Recarga**: en Unix `SIGHUP`; en otros entornos reinicia el proceso.
- **Persistencia**: SQLite en WAL con rollups y pruning por `retention_hours`; persiste `db_path` en volumen.

## Contribución y release

- Guía: [CONTRIBUTING.md](CONTRIBUTING.md). `ROADMAP.md` se mantiene local como guía de trabajo.
- Releases y artefactos: [GitHub Releases](https://github.com/dev1lsconf/tinytop/releases) — tarballs Linux (x86_64, aarch64, musl) y *BSD (`.tar.gz` + `.sha256`), paquetes nativos **.deb/.rpm** (x86_64) e imagen `ghcr.io/dev1lsconf/tinytop` (multi-arch amd64/arm64).
- Plantillas y notas de empaquetado: [`packaging/`](packaging/README.md) (Homebrew, AUR, *BSD).
- Workflow `.github/workflows/release.yml`: construye y publica artefactos, paquetes nativos e imagen multi-arch en tags `v*` (o `workflow_dispatch`).

## Changelog y licencia

- Ver [CHANGELOG.md](CHANGELOG.md).
- Licencia: MIT - Copyright (c) 2026 TinyTop Contributors.

## Agradecimientos

- [Rust](https://www.rust-lang.org), [Tokio](https://tokio.rs), [Axum](https://github.com/tokio-rs/axum), [SQLx](https://github.com/launchbadge/sqlx), [sysinfo](https://github.com/GuillaumeGomez/sysinfo), [Chart.js](https://www.chartjs.org/)
