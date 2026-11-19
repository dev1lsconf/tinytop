# TinyTop — Guía avanzada y troubleshooting

Esta guía amplía el README con detalles operativos, configuración avanzada y resolución de problemas para TinyTop 0.1.x.

## 1. Configuración

- Archivo por defecto: `config.toml` (o `--config <ruta>` / `TINYTOP_CONFIG`).
- Validar sin ejecutar: `tinytop --check-config [--config <ruta>]` (añade `--deep`, junto con `--check-config`, para verificar también la apertura/migración efectiva de `db_path`).
- Ejemplos listos para adaptar: [`examples/`](../examples/README.md).
- Overrides por entorno (útiles en contenedores): `TINYTOP_BIND`, `TINYTOP_PORT`, `TINYTOP_DB_PATH`, `TINYTOP_INTERVAL`, `TINYTOP_RETENTION_HOURS`, `TINYTOP_REQUEST_TIMEOUT_SECS`, `TINYTOP_MAX_HISTORY_HOURS`, `TINYTOP_AUTH_TOKEN`.

### 1.1 Targets y checks

Cada target define un check. Campos comunes: `name`, `check_type` (`http` por defecto, `tcp`, `ping`, `tls`), y según el tipo:

| check_type | Campos principales | Notas |
|---|---|---|
| `http`/`https` | `url`, `kind` (generic, spring-boot/actuator, quarkus, micrometer/prometheus, etc.) | Sondeo HTTP compatible con integraciones comunes; latencia y detalle en dashboard/API. |
| `tcp` | `host`, `port` | Conexión TCP; requiere host y puerto. |
| `ping` | `host` (o url/host) | **Best-effort**: depende de `ping` disponible en el sistema/contenedor. |
| `tls` | `host`, `port` | Handshake y días de certificado; **best-effort**: depende de `openssl` en el entorno. |

Umbrales por target: `latency_warn_ms`, `cert_expiry_warn_days` (opcionales).

### 1.2 Alertas

Sección `[alerts]` (umbrales globales): `cpu_percent`, `memory_percent`, `disk_percent`, `target_latency_ms`, `cert_days_remaining`. Los incidentes/eventos se registran y se exponen en `/api/alerts` y `/api/events` (y en dashboard/TUI).

### 1.3 Notificaciones

Sección `[notifications]` (best-effort, en transiciones): `webhook_url`, `telegram_bot_token` + `telegram_chat_id`, `ntfy_url`/`ntfy_topic`. Para `webhook_url` se envía JSON genérico (`{"text","alerts"}`); si la URL es un incoming webhook de Discord (`discord.com/api/webhooks/...` o `discordapp.com/api/webhooks/...`) se envía el formato compatible `{"content": ...}`. Los secretos se redactan en `/api/config`. Fallos de entrega no deben tumbar el monitoreo.

### 1.4 Autenticación

Sección `[auth]` (opcional): token (Bearer / `X-API-Key` o Basic `username`/`password`). `?token=` se acepta por compatibilidad, pero **se desaconseja**: el token puede quedar expuesto en historial del navegador, logs del proxy/servidor y cabeceras `Referer`; prefiere `Authorization: Bearer <token>`, `X-API-Key` o el botón/panel **Token** del dashboard (el panel se abre al pulsar “Token” o cuando la API responde 401; el token se guarda solo en la sesión del navegador y se envía en cabeceras, no en la URL).

Sin auth la API/UI quedan abiertas: **usar solo en red confiable o detrás de reverse proxy con TLS/auth**. `/api/health` puede quedar exento según configuración; `/api/config` redacta valores sensibles. El dashboard sirve CSS/JS como assets propios y aplica una CSP endurecida (sin `unsafe-inline`); para scripts/extensiones personalizadas externas habría que ajustar la política.

## 2. API y exportación

- `GET /api/health` — estado (db/runtime/versión).
- `GET /api/metrics/latest` — últimas métricas de sistema y targets.
- `GET /api/metrics/history?range=<h>&target=<system|nombre>` — histórico (rangos largos usan rollups/downsampling). Para targets incluye latencia, disponibilidad (`up_ratio`) y `cert_days_left` cuando aplica.
- `GET /api/insights?range=<h>` — conteos agregados por intervalo de eventos (por nivel) y alertas (por estado) para visualización.
- `GET /api/config` — configuración efectiva (redactada).
- `GET /api/alerts`, `GET /api/events` — incidentes y eventos.
- `GET /api/export?target=<...>&format=json|csv&range=<h>` — exportación.
- `GET /metrics` — métricas Prometheus.

Ejemplos:

```bash
curl -s http://localhost:9090/api/health
curl -s "http://localhost:9090/api/metrics/history?range=24&target=system"
curl -s "http://localhost:9090/api/export?target=system&format=csv&range=24" -o system.csv
curl -s http://localhost:9090/metrics
```

## 3. Dashboard y TUI

- **Dashboard web**: embebido, sin CDN; métricas extendidas e histórico por rango (sistema y targets, con downsampling), CPU vs memoria, latencia/disponibilidad de targets e historial de `cert_days_left` (TLS), eventos/alertas por intervalo, estado de targets/TLS y exportación.
- **TUI**: `tinytop --tui [--config <ruta>]` (requiere TTY). Vistas Sistema/Targets/Alertas/Eventos, sparklines, selección de rango/target y refresco. Atajos: `Tab`/`1-4`, `+/-`, `↑/↓`, `r`, `q`.

## 4. Operación

- **Instalación por plataforma**: pasos detallados para Linux, Docker, FreeBSD, OpenBSD y NetBSD en [INSTALLATION.md](INSTALLATION.md).
- **Base de datos y permisos**: la DB/esquema se crean automáticamente al iniciar si el usuario del proceso puede escribir en el directorio de `db_path` (SQLite WAL genera `*.db-wal`/`*.db-shm`). Usa `db_path` absoluto y un usuario dedicado sin privilegios (no requiere root salvo puertos <1024).
- **Recarga (Unix)**: `SIGHUP` recarga configuración sin reiniciar el proceso (según campos soportados). En otros entornos, reinicia el servicio.
- **systemd**: plantilla en [`packaging/systemd/tinytop.service`](../packaging/systemd/tinytop.service). Ajusta `User/Group`, `--config`, `db_path` (directorio escribible, p.ej. `/var/lib/tinytop/`) y permisos.
- **FreeBSD/OpenBSD/NetBSD**: plantillas rc.d y notas en [`packaging/bsd/`](../packaging/bsd/README.md). En OpenBSD (p.ej. 7.9), si el binario precompilado falla por librerías/ABI, compila nativamente (`pkg_add rust sqlite3 && cargo build --release --locked`).
- **Contenedor**: imagen `ghcr.io/dev1lsconf/tinytop` (multi-arch). Persiste `db_path` en volumen; expón el puerto configurado; monta/provee `config.toml` si hace falta.
- **Persistencia**: SQLite en WAL con migraciones, rollups y pruning por `retention_hours`. Haz backup del archivo de base de datos si lo necesitas.
- **Reverse proxy/TLS**: recomendado exponer detrás de un proxy (nginx/caddy/traefik) con TLS y, si aplica, auth adicional.

## 5. Troubleshooting

| Síntoma | Posibles causas / acciones |
|---|---|
| Config inválida al arrancar | Ejecuta `tinytop --check-config [--deep] --config <ruta>`; revisa tipos de check, `host/port` en tcp/tls, URLs y rangos. |
| Dashboard sin datos / “starting” | Espera al menos un `interval_seconds`; revisa `/api/health` y logs; confirma que el colector corre y `db_path` es escribible. Valida con `--check-config [--deep]`. |
| Check `ping` o TLS sin detalle de cert | Best-effort: instala `ping`/`openssl` en el entorno (en contenedores mínimos pueden faltar). |
| 401/403 en API/UI | Auth habilitada: usa el botón **Token** del dashboard o envía `Authorization: Bearer <token>` / `X-API-Key` / Basic según config; evita `?token=` (riesgo de filtrado). Verifica `TINYTOP_AUTH_TOKEN`/config. |
| Histórico lento o grande | Los rangos >24h usan rollups (5m/1h); ajusta `retention_hours`/`max_history_hours` y asegura espacio en disco para `db_path`. |
| Cambios de config no aplican | En Unix envía `SIGHUP` al proceso; si no, reinicia. Verifica que el servicio use el `--config` esperado. |
| Notificaciones no llegan | Best-effort y solo en transiciones; valida URL/token/chat/topic y conectividad saliente desde el host. |
| TUI no inicia | Requiere TTY interactiva; ejecuta en una terminal real (`tinytop --tui`). |
| No se crea la DB / sin datos | Verifica permisos de escritura en el directorio de `db_path` (usuario del servicio), usa ruta absoluta y revisa logs/`/api/health`. En OpenBSD, si el binario no corre por librerías/ABI, compila nativamente. |

Logs: TinyTop usa `tracing`; controla verbosidad con `RUST_LOG` (p.ej. `RUST_LOG=info,tinytop=debug`).

## 6. Compatibilidad y releases

- **Cadena automática:** Release Please (merge de su PR en `master`) crea tag/release y dispara el workflow `Release`, que publica tarballs (Linux/*BSD) + checksums, `.deb/.rpm` (x86_64) e imagen multi-arch en GHCR. La validación end-to-end real (PR→tag/release→assets/imagen) debe ejecutarse en GitHub sobre el repositorio remoto.
- **Fallback manual:** ante fallo del disparo automático o para republicar, ejecutar `Actions → Release → Run workflow` (`workflow_dispatch`) con el `tag` (p. ej. `vX.Y.Z`); el workflow hace checkout y publica contra ese tag. También puede crearse/publicarse el tag/release manualmente para activar el trigger por tag.
- Artefactos por release (tags `v*`): tarballs Linux x86_64/aarch64/musl y *BSD x86_64, paquetes `.deb/.rpm` (x86_64) e imagen `ghcr.io/dev1lsconf/tinytop` (multi-arch amd64/arm64). Ver [Releases](https://github.com/dev1lsconf/tinytop/releases) y [`packaging/`](../packaging/README.md).
- **Recetas de empaquetado:** `packaging/` (Homebrew/AUR/*BSD/systemd) son plantillas; al publicar un release deben actualizarse versión y checksums (`sha256`) según los artefactos publicados y, si cambia el runner/tooling, ajustar las recetas para mantener compatibilidad.
- Cambios y notas de versión: [CHANGELOG.md](../CHANGELOG.md).
