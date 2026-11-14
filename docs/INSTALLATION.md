# Instalación y configuración

Guía detallada para instalar, configurar y operar TinyTop en Linux, *BSD y Docker. Para dependencias por plataforma/arquitectura (librerías, ABI, opcionales) consulta [`requirements/`](../requirements/README.md).

## Índice

- [Requisitos comunes](#requisitos-comunes)
- [Configuración](#configuración)
- [Linux](#linux)
  - [Debian / Ubuntu (.deb, amd64)](#debian--ubuntu-deb-amd64)
  - [RHEL / Fedora / CentOS Stream (.rpm, x86_64)](#rhel--fedora--centos-stream-rpm-x86_64)
  - [Tarball genérico (glibc/musl, x86_64 y aarch64)](#tarball-genérico-glibcmusl-x86_64-y-aarch64)
  - [Servicio systemd](#servicio-systemd)
- [Docker](#docker)
- [FreeBSD (amd64)](#freebsd-amd64)
- [OpenBSD (amd64, ej. 7.9)](#openbsd-amd64-ej-79)
- [NetBSD (amd64)](#netbsd-amd64)
- [Compilar desde código](#compilar-desde-código)
- [Verificación](#verificación)
- [Permisos y base de datos](#permisos-y-base-de-datos)
- [Solución de problemas](#solución-de-problemas)

---

## Requisitos comunes

- **Binario:** `tinytop` compatible con la arquitectura/libc de tu sistema (ver artefactos en [Releases](https://github.com/dev1lsconf/tinytop/releases) y [`requirements/`](../requirements/README.md)).
- **Configuración:** archivo `config.toml` legible por el proceso (`--config <ruta>` o `TINYTOP_CONFIG`; por defecto `./config.toml`).
- **Base de datos (SQLite):** TinyTop crea automáticamente el archivo y el esquema (migraciones) al iniciar, **pero** el usuario del proceso debe poder **leer/escribir** en el directorio de `db_path` (SQLite usa modo WAL y genera `*.db`, `*.db-wal`, `*.db-shm`). Se recomienda una ruta absoluta.
- **Red:** puerto TCP de escucha disponible (por defecto `9090`).
- **Certificados CA:** recomendados para checks HTTPS salientes (`ca-certificates` / `ca_root_nss` / `mozilla-rootcerts` según SO).
- **TTY:** solo necesaria para el modo TUI (`--tui`).
- **Opcionales (best-effort):** `ping` en `PATH` para checks `check_type = "ping"`; `openssl` en `PATH` para obtener días restantes de certificados TLS. Si faltan, esas capacidades se omiten.
- **Privilegios:** no requiere `root`. Puede correr como usuario sin privilegios siempre que tenga permisos sobre config, directorio de DB y el puerto configurado (puertos <1024 requieren privilegios).

## Configuración

1. Copia y adapta un ejemplo: [`examples/config.basic.toml`](../examples/config.basic.toml) o [`examples/config.advanced.toml`](../examples/config.advanced.toml).
2. Ajusta como mínimo: `bind`, `port`, `db_path` (ruta absoluta recomendada), `interval_seconds`, `retention_hours` y `targets`.
3. Valida antes de ejecutar:

```bash
tinytop --check-config --config /ruta/config.toml
```

4. Ejecuta (dashboard web en `http://<bind>:<port>`, por defecto `http://localhost:9090`):

```bash
tinytop --config /ruta/config.toml
# o TUI:
tinytop --tui --config /ruta/config.toml
```

Variables de entorno útiles (sobrescriben valores del archivo): `TINYTOP_CONFIG`, `TINYTOP_BIND`, `TINYTOP_PORT`, `TINYTOP_DB_PATH`, `TINYTOP_INTERVAL`, `TINYTOP_RETENTION_HOURS`, `TINYTOP_REQUEST_TIMEOUT_SECS`, `TINYTOP_MAX_HISTORY_HOURS`, `TINYTOP_AUTH_TOKEN`. En Unix, `SIGHUP` recarga la configuración.

Más detalles: [docs/ADVANCED.md](ADVANCED.md) y [ejemplos](../examples/README.md).

---

## Linux

Requisitos detallados: [`requirements/linux-x86_64-gnu.txt`](../requirements/linux-x86_64-gnu.txt), [`requirements/linux-aarch64-gnu.txt`](../requirements/linux-aarch64-gnu.txt), [`requirements/linux-x86_64-musl.txt`](../requirements/linux-x86_64-musl.txt).

### Debian / Ubuntu (.deb, amd64)

Artefacto: `tinytop_<version>_amd64.deb` ([Releases](https://github.com/dev1lsconf/tinytop/releases)).

```bash
sudo apt-get update
sudo apt-get install -y sqlite3 libsqlite3-0 ca-certificates iputils-ping openssl
sudo dpkg -i tinytop_1.0.0_amd64.deb

# Configuración y datos
sudo install -d -o root -g root /etc/tinytop
sudo install -d -o tinytop -g tinytop /var/lib/tinytop  # si usarás el servicio dedicado
sudoedit /etc/tinytop/config.toml   # ajustar db_path, p.ej. /var/lib/tinytop/tinytop.db

tinytop --check-config --config /etc/tinytop/config.toml
```

- Binario: `/usr/bin/tinytop`. Config de ejemplo: `/etc/tinytop/config.toml`.
- Si el binario requiere `libsqlite3`, asegúrate de tener `libsqlite3-0` instalado (verifica con `ldd /usr/bin/tinytop`).

### RHEL / Fedora / CentOS Stream (.rpm, x86_64)

Artefacto: `tinytop-<version>-1.x86_64.rpm`.

```bash
sudo dnf install -y sqlite sqlite-libs ca-certificates iputils openssl
sudo rpm -i tinytop-1.0.0-1.x86_64.rpm

tinytop --check-config --config /etc/tinytop/config.toml
```

Verifica dependencias con `ldd /usr/bin/tinytop` si es necesario.

### Tarball genérico (glibc/musl, x86_64 y aarch64)

Artefactos: `tinytop-v<version>-linux-x86_64.tar.gz`, `linux-aarch64.tar.gz`, `linux-x86_64-musl.tar.gz`. Contienen `tinytop`, `config.toml`, `static/`, `migrations/`.

```bash
tar xzf tinytop-v1.0.0-linux-x86_64.tar.gz
cd tinytop-v1.0.0-linux-x86_64  # o el directorio extraído

# Dependencias runtime (ej. Debian/Ubuntu)
sudo apt-get install -y sqlite3 libsqlite3-0 ca-certificates iputils-ping openssl
# Ej. Alpine/musl: apk add --no-cache sqlite-libs ca-certificates iputils openssl

# Verificar compatibilidad/librerías
ldd ./tinytop || readelf -l ./tinytop

# Ajustar config.toml (db_path a un directorio escribible)
./tinytop --check-config --config ./config.toml
./tinytop --config ./config.toml
```

### Servicio systemd

Plantilla: [`packaging/systemd/tinytop.service`](../packaging/systemd/tinytop.service).

```bash
sudo useradd --system --home /var/lib/tinytop --shell /usr/sbin/nologin tinytop || true
sudo install -d -o tinytop -g tinytop /var/lib/tinytop
sudo install -m 0644 packaging/systemd/tinytop.service /etc/systemd/system/tinytop.service

# Editar /etc/tinytop/config.toml: db_path = "/var/lib/tinytop/tinytop.db"
sudo systemctl daemon-reload
sudo systemctl enable --now tinytop
sudo systemctl status tinytop
```

Recarga de config (Unix): `sudo systemctl reload tinytop` (SIGHUP).

---

## Docker

Requisitos: [`requirements/docker.txt`](../requirements/docker.txt). Imagen multi-arch: `ghcr.io/dev1lsconf/tinytop` (linux/amd64, linux/arm64).

```bash
docker pull ghcr.io/dev1lsconf/tinytop:1.0.0

docker run -d --name tinytop \
  -p 9090:9090 \
  -v "$PWD/config.toml:/app/config.toml:ro" \
  -v tinytop-data:/app/data \
  -e TINYTOP_CONFIG=/app/config.toml \
  -e TINYTOP_DB_PATH=/app/data/tinytop.db \
  --restart unless-stopped \
  ghcr.io/dev1lsconf/tinytop:1.0.0
```

Con Docker Compose (repositorio): `docker compose up -d --build` (ver `docker-compose.yml`).

- Persiste `db_path` en un volumen.
- **TUI:** requiere TTY: `docker exec -it tinytop tinytop --tui --config /app/config.toml`.
- Healthcheck: `GET /api/health`. Asegura que `db_path` apunte al volumen montado.

---

## FreeBSD (amd64)

Requisitos: [`requirements/freebsd-amd64.txt`](../requirements/freebsd-amd64.txt). Artefacto: `tinytop-v<version>-freebsd-x86_64.tar.gz`. Plantilla de servicio: [`packaging/bsd/tinytop.rc.freebsd`](../packaging/bsd/tinytop.rc.freebsd).

```sh
# como root
pkg install sqlite3 ca_root_nss

tar xzf tinytop-v1.0.0-freebsd-x86_64.tar.gz
install -m 0555 tinytop /usr/local/bin/tinytop
install -d /usr/local/etc/tinytop
install -m 0644 config.toml /usr/local/etc/tinytop/config.toml

# usuario y datos
pw useradd tinytop -d /var/db/tinytop -s /usr/sbin/nologin -c "TinyTop"
install -d -o tinytop -g tinytop /var/db/tinytop
# en config.toml: db_path = "/var/db/tinytop/tinytop.db"

# servicio rc.d (ajustar rutas/usuario si es necesario)
install -m 0555 packaging/bsd/tinytop.rc.freebsd /usr/local/etc/rc.d/tinytop
sysrc tinytop_enable=YES
service tinytop configtest
service tinytop start
```

- Verifica dependencias/librerías con `ldd /usr/local/bin/tinytop`. Si hay incompatibilidad de ABI/sonames con tu release, compila nativamente (ver [Compilar desde código](#compilar-desde-código)).

---

## OpenBSD (amd64, ej. 7.9)

Requisitos: [`requirements/openbsd-amd64.txt`](../requirements/openbsd-amd64.txt). Artefacto: `tinytop-v<version>-openbsd-x86_64.tar.gz`. Plantilla de servicio: [`packaging/bsd/tinytop.rc.openbsd`](../packaging/bsd/tinytop.rc.openbsd).

> **Importante:** en OpenBSD los sonames/ABI pueden variar entre releases (p. ej. 7.9). Si el binario precompilado falla por librerías no encontradas o incompatibles, instala las dependencias indicadas por `ldd` o **compila nativamente en tu release**.

### Instalación (binario precompilado)

```sh
# como root
pkg_add sqlite3

tar xzf tinytop-v1.0.0-openbsd-x86_64.tar.gz
install -m 0555 tinytop /usr/local/bin/tinytop
install -d -m 0755 /etc/tinytop
install -m 0644 config.toml /etc/tinytop/config.toml
```

### Usuario, permisos y base de datos (recomendado)

TinyTop no necesita `root`. Ejecuta como un usuario dedicado sin privilegios, con permisos de escritura en el directorio de `db_path` (SQLite crea `.db`, `.db-wal`, `.db-shm`):

```sh
# como root
useradd -L daemon -s /sbin/nologin -d /var/empty -c "TinyTop daemon" _tinytop
install -d -o _tinytop -g _tinytop -m 0750 /var/db/tinytop
```

Edita `/etc/tinytop/config.toml` y usa una ruta absoluta para la DB:

```toml
bind = "0.0.0.0"
port = 9090
db_path = "/var/db/tinytop/tinytop.db"
# ... targets ...
```

Prueba como el usuario del servicio:

```sh
doas -u _tinytop /usr/local/bin/tinytop --check-config --config /etc/tinytop/config.toml
doas -u _tinytop /usr/local/bin/tinytop --config /etc/tinytop/config.toml
```

### Servicio rc.d

```sh
# como root (ajusta daemon_user en el script si usas _tinytop)
install -m 0555 packaging/bsd/tinytop.rc.openbsd /etc/rc.d/tinytop
# editar /etc/rc.d/tinytop: daemon_user="_tinytop" y daemon_flags="--config /etc/tinytop/config.toml"
rcctl enable tinytop
rcctl start tinytop
rcctl check tinytop
# recarga: rcctl reload tinytop (SIGHUP) / validar: rcctl configtest tinytop (según script)
```

### Si el binario no funciona en OpenBSD 7.9 (librerías/ABI)

1. `pkg_add sqlite3` y verifica: `ldd /usr/local/bin/tinytop`.
2. Si falta una librería o el soname no coincide con tu release (p. ej. `libsqlite3`, libc), **compila nativamente en tu OpenBSD**:
   ```sh
   pkg_add rust sqlite3
   cargo build --release --locked
   ./target/release/tinytop --version
   ```
3. Reemplaza el binario por el compilado local y reintenta.

Más detalles: [`requirements/openbsd-amd64.txt`](../requirements/openbsd-amd64.txt) y [`packaging/bsd/README.md`](../packaging/bsd/README.md).

---

## NetBSD (amd64)

Requisitos: [`requirements/netbsd-amd64.txt`](../requirements/netbsd-amd64.txt). Artefacto: `tinytop-v<version>-netbsd-x86_64.tar.gz`. Plantilla: [`packaging/bsd/tinytop.rc.netbsd`](../packaging/bsd/tinytop.rc.netbsd).

```sh
# como root
pkg_add sqlite3 mozilla-rootcerts
/usr/sbin/mozilla-rootcerts install || true

tar xzf tinytop-v1.0.0-netbsd-x86_64.tar.gz
install -m 0555 tinytop /usr/pkg/bin/tinytop || install -m 0555 tinytop /usr/local/bin/tinytop
install -d /usr/pkg/etc/tinytop
install -m 0644 config.toml /usr/pkg/etc/tinytop/config.toml

install -d -o tinytop -g tinytop /var/db/tinytop
# config.toml: db_path = "/var/db/tinytop/tinytop.db"
# instalar rc script y habilitar según pkgsrc/rc.d local
```

Verifica con `ldd ./tinytop`. Si hay incompatibilidad de ABI, compila nativamente (ver [Compilar desde código](#compilar-desde-código)).

---

## Compilar desde código

Requisitos: [`requirements/build-from-source.txt`](../requirements/build-from-source.txt).

Herramientas: Rust **1.88+** (edition 2024), `cargo`, compilador/enlazador C, `pkg-config`, headers/librería de **SQLite3**.

```bash
# Ej. Debian/Ubuntu
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libsqlite3-dev curl ca-certificates
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain 1.88
source "$HOME/.cargo/env"

# Compilar
cargo build --release --locked
./target/release/tinytop --check-config
./target/release/tinytop --version
```

- En *BSD instala `rust`/toolchain y `sqlite3` vía paquetes del sistema (en OpenBSD/NetBSD se recomienda compilar en la misma release destino para compatibilidad ABI). Las dependencias de crates se resuelven con `Cargo.lock`.

---

## Verificación

```bash
# Versión y config
tinytop --version
tinytop --check-config --config /ruta/config.toml

# Ejecución (web)
tinytop --config /ruta/config.toml
# en otra terminal:
curl -s http://localhost:9090/api/health

# TUI (requiere TTY)
tinytop --tui --config /ruta/config.toml
```

`/api/health` con `db.ok: true` y `ready: true` indica que la base de datos y el runtime están operativos.

## Permisos y base de datos

- **Creación automática:** la DB y el esquema se crean al iniciar si el proceso puede escribir en `db_path` y su directorio.
- **Escritura requerida:** el usuario del proceso debe tener permisos de lectura/escritura en el directorio de `db_path` (WAL crea archivos auxiliares).
- **Ruta absoluta recomendada:** especialmente en servicios (systemd/rc.d), para no depender del directorio de trabajo.
- **Puerto privilegiado:** bind en `<1024` requiere privilegios (evítalo; usa 9090 u otro >=1024).
- **Sin root:** ejecuta con usuario dedicado (`tinytop`, `_tinytop`, etc.) con acceso solo a config (lectura) y directorio de datos (escritura).

## Solución de problemas

| Síntoma | Causa probable / acción |
|---|---|
| No se crea la DB / `unable to open database file` / `readonly database` | `db_path` apunta a un directorio no escribible o inexistente sin permisos para crearlo. Usa ruta absoluta, crea el directorio y asigna dueño/permisos al usuario del servicio. |
| Dashboard/TUI sin datos (CPU/RAM vacíos) | Espera 1–2 `interval_seconds`; revisa `/api/health` (`collections_total`, `last_collection_ok/error`), logs y que `db_path` sea escribible. |
| `permission denied` al iniciar | Sin permisos de lectura en binario/config, o escritura en directorio de DB, o bind a puerto <1024. |
| Binario no ejecuta en *BSD / librerías faltantes | `ldd ./tinytop`; instala dependencias (`sqlite3`, CA). Si el soname/ABI no coincide con tu release (p. ej. OpenBSD 7.9), compila nativamente. |
| Check `ping` o días de cert TLS sin datos | Best-effort: instala `ping`/`openssl` en el entorno. |
| 401/403 en API/UI | Auth habilitada: envía token/Basic según config. |
| TUI no inicia | Requiere TTY interactiva. |
| Cambios de config no aplican | Envía `SIGHUP` (Unix) o reinicia el servicio. |

Logs: controla verbosidad con `RUST_LOG` (p. ej. `RUST_LOG=info,tinytop=debug`).

Ver también: [docs/ADVANCED.md](ADVANCED.md), [`requirements/`](../requirements/README.md) y [`packaging/`](../packaging/README.md).
