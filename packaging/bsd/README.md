# Packaging notes for *BSD

TinyTop targets FreeBSD, OpenBSD and NetBSD (via `sysinfo`) and the release workflow publishes x86_64 tarballs for each (`tinytop-v<version>-<os>-x86_64.tar.gz`).

This directory contains template rc/service scripts as a starting point for native packages:
- `tinytop.rc.freebsd` — FreeBSD `rc.d` script (daemonized, `reload` via `SIGHUP`, `configtest`).
- `tinytop.rc.openbsd` — OpenBSD `rc.d` script.
- `tinytop.rc.netbsd` — NetBSD `rc.d` script.

Each tarball contains: `tinytop` binary, `config.toml`, `static/`, `migrations/`.

## FreeBSD (ports/pkg)
- `DISTFILES`: use the release tarball `tinytop-v${VERSION}-freebsd-x86_64.tar.gz` (or build from source with Rust/cargo).
- Install paths (suggested):
  - `${PREFIX}/bin/tinytop`
  - `${PREFIX}/etc/tinytop/config.toml` (sample, `@sample`)
  - `${PREFIX}/share/doc/tinytop/README.md`, `LICENSE` (optional)
  - `static/` and `migrations/` under `${DATADIR}/tinytop/` if the port installs assets separately (the binary serves embedded/static assets as packaged in the tarball; keep layout consistent with your package).
- Service: install `tinytop.rc.freebsd` as `${PREFIX}/etc/rc.d/tinytop` (mode 0555). Create an unprivileged user/group `tinytop`.
- Data: persist `db_path` under a writable directory, e.g. `/var/db/tinytop/tinytop.db` (see `WorkingDirectory`/`ReadWritePaths` equivalent via ownership of `/var/db/tinytop`).
- `rc.conf` variables (defaults in script): `tinytop_enable`, `tinytop_user`, `tinytop_group`, `tinytop_config`, `tinytop_db_dir`, `tinytop_flags`.
- Validate: `service tinytop configtest`; reload: `service tinytop reload` (SIGHUP, Unix).

## OpenBSD (ports/pkg)
- `DISTFILES`: `tinytop-v${VERSION}-openbsd-x86_64.tar.gz` or build with cargo/rust.
- Install: `/usr/local/bin/tinytop`, sample config `/etc/tinytop/config.toml`, rc script `tinytop.rc.openbsd` as `/etc/rc.d/tinytop`.
- Create `_tinytop`/`tinytop` user as per port conventions, ensure `db_path` directory is writable (e.g. `/var/db/tinytop`).
- Enable: `rcctl enable tinytop`; check: `rcctl check tinytop`; reload supported via `SIGHUP`/`rcctl reload` (template sets `rc_reload=YES`).

## NetBSD (pkgsrc)
- Use `tinytop-v${VERSION}-netbsd-x86_64.tar.gz` or cargo build in pkgsrc.
- Install: `${PREFIX}/bin/tinytop`, config under `${PKG_SYSCONFDIR}/tinytop/config.toml`, rc script `tinytop.rc.netbsd` into `${RCD_SCRIPTS_DIR}`.
- Persist `db_path` in a writable location (e.g. `/var/db/tinytop`); run as unprivileged user.

## General notes
- Validate config before start/reload: `tinytop --check-config --config <path>`.
- The web dashboard listens on configured `bind`/`port` (default `0.0.0.0:9090`). TUI requires a TTY (`tinytop --tui`).
- `ping` and TLS certificate day checks are best-effort and depend on `ping`/`openssl` being available.
- See also [`../systemd/tinytop.service`](../systemd/tinytop.service) for Linux/systemd and [`../../docs/ADVANCED.md`](../../docs/ADVANCED.md) for operational guidance.
