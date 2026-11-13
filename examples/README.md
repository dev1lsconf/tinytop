# Ejemplos de configuración

- `config.basic.toml`: monitor de sistema + targets HTTP mínimos.
- `config.advanced.toml`: checks `http`/`tcp`/`ping`/`tls`, alertas, notificaciones y auth opcional (comentados como plantilla).

Uso:

```bash
./tinytop --check-config --config examples/config.basic.toml
./tinytop --config examples/config.basic.toml
```

Ajusta `db_path`, puertos, hosts y credenciales a tu entorno. Los checks `ping` y días de certificado TLS son best-effort (requieren `ping`/`openssl`). No commitees secretos reales.
