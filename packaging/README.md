# Empaquetado y distribución

TinyTop publica artefactos en [GitHub Releases](https://github.com/dev1lsconf/tinytop/releases) desde tags `v*` (workflow `.github/workflows/release.yml`).

## Artefactos publicados por release

- **Linux** (tarball `.tar.gz` + `.sha256`): `linux-x86_64` (gnu), `linux-aarch64` (gnu), `linux-x86_64-musl`.
- **Paquetes nativos Linux (x86_64)**: `.deb` (cargo-deb) y `.rpm` (cargo-generate-rpm), generados en el build nativo gnu.
- **BSD** (tarball `.tar.gz` + `.sha256`): `freebsd`, `openbsd`, `netbsd` (x86_64).
- **Contenedor**: imagen multi-arch `ghcr.io/dev1lsconf/tinytop` (linux/amd64, linux/arm64), tags de versión y `latest`.

Cada tarball incluye: binario `tinytop`, `config.toml`, `static/` y `migrations/`.

## Plantillas en este repositorio

- `packaging/homebrew/tinytop.rb`: fórmula de ejemplo (URLs de release; **reemplazar `sha256`** por los checksums publicados al liberar).
- `packaging/aur/PKGBUILD`: receta AUR de ejemplo (x86_64/aarch64, tarballs Linux; **reemplazar `sha256sums_*`**).
- `packaging/bsd/README.md`: notas/guía para ports/pkg (FreeBSD), ports (OpenBSD) y pkgsrc (NetBSD).

- `packaging/systemd/tinytop.service`: unidad systemd de ejemplo (usuario dedicado, recarga por `SIGHUP`, hardening básico).

Las plantillas usan la versión actual del proyecto; al publicar un release, actualiza `version/pkgver` y los hashes.

## Instalación desde paquetes nativos (Linux x86_64)

```bash
# Debian/Ubuntu (.deb descargado del release)
sudo dpkg -i tinytop_<version>_amd64.deb

# RHEL/Fedora (.rpm descargado del release)
sudo rpm -i tinytop-<version>-1.x86_64.rpm
```

- Binario: `/usr/bin/tinytop`
- Config de ejemplo: `/etc/tinytop/config.toml`
- Docs/licencia: `/usr/share/doc/tinytop/`

### Servicio systemd (ejemplo)

```bash
sudo useradd --system --home /var/lib/tinytop --shell /usr/sbin/nologin tinytop || true
sudo install -d -o tinytop -g tinytop /var/lib/tinytop
sudo install -m 0644 packaging/systemd/tinytop.service /etc/systemd/system/tinytop.service
# Ajusta /etc/tinytop/config.toml (db_path en /var/lib/tinytop/...)
sudo systemctl daemon-reload && sudo systemctl enable --now tinytop
```

Valida la configuración antes de ejecutar: `tinytop --check-config --config /etc/tinytop/config.toml`.

## Notas

- `ping` y la obtención de días de certificado TLS son best-effort y dependen de `ping`/`openssl` en el entorno.
- La TUI requiere TTY. La recarga de config por `SIGHUP` está disponible en Unix.
