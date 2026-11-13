# Contribuir a TinyTop

¡Gracias por tu interés en contribuir a TinyTop! El objetivo del proyecto es ser un monitor autohospedado **ligero, de un solo binario y con mínimas dependencias externas**.

## Cómo empezar

1. Haz fork del repositorio y crea una rama desde `master`.
2. Requisitos: Rust estable (ver `rust-toolchain`/CI, actualmente 1.88+), y opcionalmente Node.js para las pruebas e2e con Playwright.
3. Compila y ejecuta localmente:

   ```bash
   cargo build --release
   ./target/release/tinytop --check-config
   ./target/release/tinytop
   # o la TUI
   ./target/release/tinytop --tui
   ```

## Calidad y pruebas

Antes de abrir un PR, asegúrate de que todo pasa:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
```

Las pruebas e2e (Playwright) viven en `tests/` y se ejecutan en CI; localmente puedes instalarlas con `npm install` y `npx playwright install --with-deps chromium`.

## Guías

- Mantén la filosofía: binario único, bajo consumo y sin dependencias externas obligatorias en runtime (las integraciones opcionales deben ser best-effort).
- Sigue el estilo y patrones existentes; evita añadir dependencias innecesarias.
- Incluye pruebas para cambios de comportamiento (API, checks, alertas, rollups, auth/export cuando aplique).
- Documenta flags, configuración y endpoints nuevos en `README.md`/`CHANGELOG.md`.
- No incluyas secretos ni configuración local (`config.toml`/`.commandcode/settings.json` no deben commitearse con datos personales).

## Pull requests

- Describe el problema y la solución, y cómo probarlo.
- Prefiere cambios pequeños y enfocados.
- Asegúrate de que el CI pasa (backend + e2e).

## Issues

Los issues con etiqueta `good first issue` son un buen punto de partida. Si no estás seguro del enfoque, abre un issue para discutirlo antes de implementar cambios grandes.
