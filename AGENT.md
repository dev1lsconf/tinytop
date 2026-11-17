# AGENT.md - Notas para agentes

## UI y pruebas E2E (Playwright)
- **Siempre** que se modifique la UI (HTML/CSS/JS del dashboard, `static/*`, layout, tarjetas, gráficos o interacciones visibles), se debe **revisar y actualizar las pruebas Playwright** (`tests/ui.spec.ts`, `playwright.config.ts`) y ejecutarlas/validarlas según corresponda. Los cambios de UI no se consideran completos sin revisión de e2e.
- **Layout de tarjetas/cards:** las tarjetas deben verse alineadas y “cuadradas” (misma altura/ancho dentro de cada fila de grid, sin huecos desparejos). Al quitar o agregar métricas (p. ej. swap/disco), ajustar la grilla (`.stats`, `.kv`, charts) para que las cards queden parejas y cubrirlo en Playwright (asserts de layout/alineación).
- Ejecutar/validar: e2e con Playwright (`npx playwright test` o el flujo de CI con servidor en marcha) además de `cargo test --all-targets` cuando aplique.
