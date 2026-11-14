# Requisitos y dependencias por arquitectura

Archivos de requisitos (estilo `requirements.txt`) para instalar/ejecutar TinyTop en cada plataforma soportada. Usa el archivo que corresponda a tu sistema/artefacto para **asegurar una implementación sin problemas**.

## Archivos

| Archivo | Uso |
|---|---|
| [`common.txt`](common.txt) | Requisitos comunes (runtime y notas generales) |
| [`linux-x86_64-gnu.txt`](linux-x86_64-gnu.txt) | Binario/tarball `linux-x86_64` (glibc) |
| [`linux-aarch64-gnu.txt`](linux-aarch64-gnu.txt) | Binario/tarball `linux-aarch64` (glibc) |
| [`linux-x86_64-musl.txt`](linux-x86_64-musl.txt) | Binario/tarball `linux-x86_64-musl` |
| [`debian-ubuntu-amd64.txt`](debian-ubuntu-amd64.txt) | Paquete `.deb` (amd64) |
| [`rhel-fedora-x86_64.txt`](rhel-fedora-x86_64.txt) | Paquete `.rpm` (x86_64) |
| [`freebsd-amd64.txt`](freebsd-amd64.txt) | Tarball/ports FreeBSD x86_64 |
| [`openbsd-amd64.txt`](openbsd-amd64.txt) | Tarball/ports OpenBSD x86_64 (ej. 7.9) |
| [`netbsd-amd64.txt`](netbsd-amd64.txt) | Tarball/pkgsrc NetBSD x86_64 |
| [`docker.txt`](docker.txt) | Contenedor (GHCR / Dockerfile) |
| [`build-from-source.txt`](build-from-source.txt) | Compilar desde código (todas las plataformas) |

## Notas rápidas

- Las dependencias de crates Rust se resuelven en compilación (`Cargo.toml`/`Cargo.lock`). TLS HTTP usa **rustls** (no requiere OpenSSL para HTTP).
- El binario necesita una libc/loader compatible con la plataforma del artefacto. En algunos builds puede requerir `libsqlite3` del sistema: verifica con `ldd ./tinytop` (Linux/*BSD donde aplique) y revisa el archivo de tu plataforma.
- Checks **best-effort**: `ping` (check `ping`) y `openssl` (días de certificado TLS) son opcionales; si faltan, esos datos no estarán disponibles pero el resto funciona.
- La **TUI** requiere una terminal interactiva (TTY).
