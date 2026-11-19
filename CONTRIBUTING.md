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
cargo audit # requiere cargo-audit instalado: cargo install cargo-audit --locked
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

## Releases (mantenedores)

- La cadena automática es: merge del PR de Release Please en `master` → tag/release → workflow `Release` (publica binarios/paquetes/imagen). La validación end-to-end de esta cadena se realiza en GitHub sobre el repositorio remoto.
- **Fallback manual:** si el disparo automático falla o para republicar, ejecuta `Actions → Release → Run workflow` (`workflow_dispatch`) indicando el `tag` (p. ej. `vX.Y.Z`); el workflow hace checkout y publica contra ese tag. También puede crearse/publicarse el tag/release manualmente para activar el trigger por tag.
- En cada release, alinea `packaging/` (versión y checksums `sha256`/`sha256sums`) con los artefactos publicados en GitHub Releases (no inventar hashes). Mantén las actions/toolchains pineadas y revisa avisos de deprecación (runners/Node en actions) según corresponda.

## Issues

Los issues con etiqueta `good first issue` son un buen punto de partida. Si no estás seguro del enfoque, abre un issue para discutirlo antes de implementar cambios grandes.
