# Packaging notes for *BSD

TinyTop targets FreeBSD, OpenBSD and NetBSD (via `sysinfo`) and the release workflow publishes x86_64 tarballs for each (`tinytop-v<version>-<os>-x86_64.tar.gz`).

These notes are templates/guidance for native packages:

## FreeBSD (ports/pkg)
- Create a port using the release tarball as `DISTFILES` (or build from source with Rust/cargo).
- Install: `tinytop` -> `${PREFIX}/bin/tinytop`, `config.toml` -> `${PREFIX}/etc/tinytop/config.toml` (sample), include `static/` and `migrations/` under `${DATADIR}` if packaging assets separately.
- Provide an `rc.d` service script running `tinytop --config /usr/local/etc/tinytop/config.toml` as an unprivileged user and persisting `db_path` under `/var/db/tinytop/`.
- Runtime: `sqlite3`/`curl` as applicable; checks `ping`/`openssl` are best-effort.

## OpenBSD (ports/pkg)
- Similar to FreeBSD: port fetching the OpenBSD release tarball or building with cargo/rust.
- Install rc.d script (`/etc/rc.d/tinytop`) and sample config in `/etc/tinytop/config.toml`.

## NetBSD (pkgsrc)
- pkgsrc package using the NetBSD release tarball or cargo build.
- Install rc.d script and config under `${PKG_SYSCONFDIR}`.

## General
- Tarballs contain: `tinytop` binary, `config.toml`, `static/`, `migrations/`.
- Validate config with `tinytop --check-config --config <path>`; reload on Unix with `SIGHUP`.
- TUI requires a TTY; web dashboard listens on configured bind/port (default 9090).
