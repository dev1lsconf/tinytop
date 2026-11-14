# TinyTop v1.0.1 - Monitor ligero en Rust

[![CI](https://github.com/dev1lsconf/tinytop/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/dev1lsconf/tinytop/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/dev1lsconf/tinytop?display_name=tag)](https://github.com/dev1lsconf/tinytop/releases)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![version](https://img.shields.io/badge/version-1.0.1-blue.svg)](Cargo.toml)
[![rust](https://img.shields.io/badge/Rust-1.88%2B%20(edition%202024)-orange.svg)](https://www.rust-lang.org)
[![platform](https://img.shields.io/badge/platform-linux%20%7C%20BSD-lightgrey.svg)](https://www.kernel.org)

![TinyTop Dashboard](docs/dashboard.png?v=1.0.1)

**TUI (terminal):**

![TinyTop TUI](docs/tui.gif)

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

`config.toml` (ejemplo, ver más en [`examples/`](examples/README.md)):

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

## Dependencias

TinyTop es un binario único escrito en Rust (edition 2024). Las dependencias de crates se gestionan con Cargo (`Cargo.toml`/`Cargo.lock`, `--locked`); **no hay `requirements.txt` de Python/pip**: los requisitos por plataforma/arquitectura (equivalentes para instalación y despliegue) están en [`requirements/`](requirements/README.md).

### Requisitos por plataforma y arquitectura (instalación/ejecución)

| Plataforma / artefacto | Archivo de requisitos |
|---|---|
| Común (runtime) | [`requirements/common.txt`](requirements/common.txt) |
| Linux x86_64 (glibc, tarball) | [`requirements/linux-x86_64-gnu.txt`](requirements/linux-x86_64-gnu.txt) |
| Linux aarch64 (glibc, tarball) | [`requirements/linux-aarch64-gnu.txt`](requirements/linux-aarch64-gnu.txt) |
| Linux x86_64 (musl, tarball) | [`requirements/linux-x86_64-musl.txt`](requirements/linux-x86_64-musl.txt) |
| Debian/Ubuntu amd64 (.deb) | [`requirements/debian-ubuntu-amd64.txt`](requirements/debian-ubuntu-amd64.txt) |
| RHEL/Fedora x86_64 (.rpm) | [`requirements/rhel-fedora-x86_64.txt`](requirements/rhel-fedora-x86_64.txt) |
| FreeBSD amd64 | [`requirements/freebsd-amd64.txt`](requirements/freebsd-amd64.txt) |
| OpenBSD amd64 (ej. 7.9) | [`requirements/openbsd-amd64.txt`](requirements/openbsd-amd64.txt) |
| NetBSD amd64 | [`requirements/netbsd-amd64.txt`](requirements/netbsd-amd64.txt) |
| Docker (linux/amd64, linux/arm64) | [`requirements/docker.txt`](requirements/docker.txt) |
| Compilar desde código (todas) | [`requirements/build-from-source.txt`](requirements/build-from-source.txt) |

### Dependencias de ejecución (runtime)

- **Binario**: `tinytop` con loader/libc compatibles con el artefacto de la plataforma (glibc, musl o libc del *BSD). Verifica con `ldd ./tinytop` (donde aplique).
- **SQLite3**: el binario puede requerir `libsqlite3` del sistema según el artefacto/build (p.ej. en *BSD es habitual). Si falta o el soname no coincide con tu release, instálala (`pkg_add sqlite3` en OpenBSD, `pkg install sqlite3` en FreeBSD, etc.) o compila nativamente (ver requisitos de tu plataforma).
- **Certificados raíz (CA)**: recomendados para checks HTTPS salientes (`ca-certificates` / `ca_root_nss` / `mozilla-rootcerts` según sistema).
- **Archivos y permisos**: `config.toml` legible y directorio de `db_path` escribible (SQLite en modo WAL). Puerto TCP de escucha disponible (9090 por defecto). **TTY** solo si usas `--tui`.
- **Opcionales, best-effort** (si faltan, solo se omiten esas capacidades):
  - `ping` en `PATH`: necesario para `check_type = "ping"` (latencia/estado ICMP).
  - `openssl` en `PATH`: necesario para obtener los días restantes de certificado en checks `tls` (el handshake TLS usa el stack del sistema/rustls según check; el cálculo de expiración es best-effort vía CLI).

### Dependencias para compilar desde código

- **Rust 1.88+** (edition 2024), `cargo`, compilador/enlazador C (`gcc`/`clang`), `pkg-config`.
- **SQLite3 (desarrollo)**: headers y librería (`libsqlite3-dev` / `sqlite3-devel` / `sqlite3`) para enlazar `libsqlite3-sys`.
- Comandos: `cargo build --release --locked` (ver [`requirements/build-from-source.txt`](requirements/build-from-source.txt) por SO).
- Crates principales (resueltas por Cargo): `tokio`, `axum`, `reqwest` (rustls-tls), `sqlx` (sqlite), `sysinfo`, `ratatui`/`crossterm`, `serde`, `tracing`, `chrono`, `toml`.

### Dependencias de desarrollo y pruebas

- **Calidad**: `rustfmt`, `clippy` (toolchain Rust).
- **E2E (Playwright, dashboard web)**: Node.js + `npm ci` y `npx playwright install --with-deps chromium` (ver [Pruebas y calidad](#pruebas-y-calidad)).

> **OpenBSD 7.9 (y otros *BSD):** si el binario precompilado falla por librerías no encontradas o ABI incompatible, instala `sqlite3` y verifica con `ldd ./tinytop`; si el soname no coincide con tu release, **compila nativamente** en tu sistema (`pkg_add rust sqlite3 && cargo build --release --locked`). Detalles en [`requirements/openbsd-amd64.txt`](requirements/openbsd-amd64.txt) y [`packaging/bsd/README.md`](packaging/bsd/README.md).

## Instalación

Guía detallada por plataforma (Linux, Docker, FreeBSD, OpenBSD, NetBSD, compilación desde código): **[docs/INSTALLATION.md](docs/INSTALLATION.md)**. Requisitos por arquitectura: [`requirements/`](requirements/README.md).

| Plataforma | Referencia rápida |
|---|---|
| Debian/Ubuntu (amd64, `.deb`) | [guía](docs/INSTALLATION.md#debian--ubuntu-deb-amd64) · [requisitos](requirements/debian-ubuntu-amd64.txt) |
| RHEL/Fedora (x86_64, `.rpm`) | [guía](docs/INSTALLATION.md#rhel--fedora--centos-stream-rpm-x86_64) · [requisitos](requirements/rhel-fedora-x86_64.txt) |
| Linux tarball (gnu/musl) | [guía](docs/INSTALLATION.md#tarball-genérico-glibcmusl-x86_64-y-aarch64) · [requisitos](requirements/linux-x86_64-gnu.txt) |
| Docker | [guía](docs/INSTALLATION.md#docker) · [requisitos](requirements/docker.txt) |
| FreeBSD (amd64) | [guía](docs/INSTALLATION.md#freebsd-amd64) · [requisitos](requirements/freebsd-amd64.txt) |
| OpenBSD (amd64, ej. 7.9) | [guía](docs/INSTALLATION.md#openbsd-amd64-ej-79) · [requisitos](requirements/openbsd-amd64.txt) |
| NetBSD (amd64) | [guía](docs/INSTALLATION.md#netbsd-amd64) · [requisitos](requirements/netbsd-amd64.txt) |
| Compilar desde código | [guía](docs/INSTALLATION.md#compilar-desde-código) · [requisitos](requirements/build-from-source.txt) |

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
- En otras plataformas soportadas por las dependencias se puede compilar desde código (`cargo build --release --locked`). Ver [instalación](docs/INSTALLATION.md) y [requisitos](requirements/README.md).

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
- **Ejemplos**: [`examples/`](examples/README.md) incluye configs básica y avanzada (targets tcp/ping/tls/http, alertas, notificaciones, auth).
- **Guía avanzada**: [docs/ADVANCED.md](docs/ADVANCED.md) (checks/alertas/notificaciones/auth, API/export, operación, troubleshooting).

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
