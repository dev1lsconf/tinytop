# TinyTop — Propuestas de mejora

Documento de propuestas para futuras versiones de TinyTop. **No compromete implementación**: cada propuesta debe priorizarse, refinarse y aprobarse antes de desarrollarse (especialmente cambios de alcance de UI/dashboard).

## Principios y restricciones

- **Filosofía:** binario único, autohospedado, sin dependencias externas obligatorias en runtime (integraciones opcionales best-effort).
- **Recursos:** objetivo de bajo consumo (**~5–50 MB de RAM** en operación típica), latencia y carga de DB acotadas.
- **Dashboard (alcance fijo y aprobado):** gráficos históricos solo **CPU**, **memoria**, **red (RX/TX)** y **load average (1/5/15)**. No añadir otros gráficos/visualizaciones ni endpoints/UI asociados sin aprobación explícita previa. Las mejoras de visibilidad deben priorizar KPIs numéricos, tablas, detalle textual o sparklines donde corresponda (p. ej. TUI) sin ampliar el alcance de gráficos del dashboard.
- **Compatibilidad:** cambios incrementales y compatibles hacia atrás cuando sea posible; migraciones SQLite seguras; configuración existente válida salvo evolución explícita y documentada.
- **Calidad:** pruebas (unitarias/integración/e2e según aplique), validación (`--check-config`), documentación (README/docs/ejemplos/changelog) y actualización de roadmap al implementar.

---

## 1. Propuestas registradas (base 1.1 / 1.2 / 2.0)

### P1. SLO y latencia agregada por target (v1.1 — alto impacto, esfuerzo bajo/medio)
- **Descripción:** calcular y exponer por ventana (p. ej. 24h/7d/30d) `uptime %`, checks ok/total, fallos, latencia agregada (p50/p95/max), último estado/cambio y alertas firing por target. Endpoint sugerido `GET /api/targets/summary?range=...` y enriquecimiento de la tabla de targets y/o KPIs numéricos.
- **Valor:** pasa de “estado actual” a fiabilidad medible; facilita diagnosticar degradaciones e incidentes.
- **Alcance sugerido:** agregados en DB (`src/db.rs`, apoyándose en métricas/rollups), ruta en `src/web/routes.rs`, UI en `static/*` (tabla/KPIs, sin gráfico histórico nuevo).
- **Consideraciones:** acotar ventanas y coste de consulta (índices/rollups, tiempos acotados); compatible hacia atrás y aditivo.

### P2. Gestión de alertas accionable: ack, silencios y deduplicación (v1.1 — alto impacto, esfuerzo medio)
- **Descripción:** `ack` de alertas (quién/cuándo/nota), silencios/mantenimientos por target o regla con expiración, deduplicación por fingerprint (source+name+severidad) para no duplicar firing abiertas y cooldown por canal+regla. Notificar solo transiciones relevantes y suprimir durante silencios.
- **Valor:** reduce ruido y convierte alertas en un flujo operable.
- **Alcance sugerido:** migraciones SQLite, lógica en alertas/notificaciones, evolución de `/api/alerts` (listado + acciones) y UI de tabla (estado/acciones).
- **Consideraciones:** diseño de estado/historial y migración cuidadosa; mantener formato actual añadiendo campos/acciones opcionales.

### P3. Visibilidad de disco y capacidad por filesystem, sin gráfico nuevo (v1.1 — alto impacto, esfuerzo bajo)
- **Descripción:** exponer y mostrar uso por mount/filesystem (usado/total/% y peor caso) como tarjetas/KPIs numéricos y/o tabla (detalle en `latest`, endpoint dedicado o sección colapsable), con tendencia expresada como delta en ventana (texto), no como gráfico.
- **Valor:** el disco lleno es causa crítica de caída y hoy tiene poca visibilidad; aprovecha datos ya recolectados.
- **Alcance sugerido:** colector/DB/API/UI (`src/collector/*`, `src/db.rs`, `src/web/routes.rs`, `static/*`). **Sin gráfico histórico de disco**.
- **Consideraciones:** deduplicar/limitar mounts para no crecer en memoria/DB; cambio aditivo.

### P4. Checks extensibles y más potentes (v1.2 — alto impacto, esfuerzo medio)
- **Descripción:** ampliar checks: HTTP enriquecido (`method`, `headers`, `body`, `expected_status`/rango, `expected_body_contains` acotado) y, como opción, `check_type = "exec"` (comando/script con timeout estricto, salida acotada y código de salida), opt-in/desactivable. Mejorar `detail` clasificando fallos (DNS, conexión, TLS, timeout, status/contenido inesperado).
- **Valor:** cubre validaciones reales sin depender de servicios externos.
- **Alcance sugerido:** `src/config.rs` (validación), `src/collector/*` (ejecución con límites), DB/UI/API (campos y detalle). Nuevos campos opcionales.
- **Consideraciones:** no filtrar secretos, acotar tamaño/tiempo, documentar riesgos de `exec` y mantener ejecución no bloqueante/best-effort.

### P5. Actualizaciones en vivo (SSE) y control de carga (v1.2 — impacto medio/alto, esfuerzo medio)
- **Descripción:** `GET /api/stream` (SSE) con snapshots/deltas de `latest`, alertas y eventos; el dashboard se suscribe con fallback a polling. Intervalos por target y jitter/muestreo configurable para repartir carga.
- **Valor:** UI más reactiva con menos consultas repetidas; ayuda a sostener 5–50 MB con más targets/rangos.
- **Alcance sugerido:** `src/web/routes.rs` y estado en memoria acotado, `static/app.js` (reconexión/fallback), `src/config.rs`/collector (intervalo/jitter).
- **Consideraciones:** manejo de conexiones, backpressure y auth en SSE; evitar buffers sin límite; compatibilidad con endpoints actuales.

### P6. Hardening de autenticación y auditoría (v1.2 — alto impacto, esfuerzo medio)
- **Descripción:** múltiples credenciales/tokens con alcance `read`/`admin` (compatible con `auth.token` actual), rate limiting en memoria para auth/endpoints sensibles, opción para deshabilitar `?token=` por defecto (manteniendo compatibilidad configurable) y eventos/auditoría de accesos fallidos y cambios relevantes.
- **Valor:** seguridad práctica en despliegues sin reverse proxy, sin dependencias externas.
- **Alcance sugerido:** `src/config.rs`, middleware web, logging/eventos y documentación/ejemplos.
- **Consideraciones:** defaults seguros y migración suave; retrocompatibilidad mapeando `auth.token` a credencial admin.

### P7. Modo multi-instancia ligero, opcional (v2.0 — impacto alto, esfuerzo alto)
- **Descripción:** el mismo binario puede, de forma opt-in, agregar (pull HTTP autenticado) otras instancias TinyTop como “nodos” y mostrar estado/alertas/resúmenes consolidados en tablas y API.
- **Valor:** visibilidad de varios hosts sin adoptar Prometheus ni un stack pesado.
- **Alcance sugerido:** configuración de nodos, cliente de agregación, API/UI de vista consolidada y manejo de fallos parciales.
- **Consideraciones:** acotar impacto en RAM/latencia, timeouts y degradación ante nodos caídos; feature mayor y desactivada por defecto.

---

## 2. Nuevas propuestas de mejora

### N1. TUI: navegación avanzada, scroll y panel de detalle
- **Descripción:** agregar scroll vertical con indicador de posición en tablas (Targets/Alertas/Eventos), selección persistente en todas las vistas y panel/modal de detalle (`Enter`) con información completa (mensaje, `detail`, metadatos, timestamps, origen/severidad, último error). Atajos sugeridos: `j/k` o flechas, `PgUp/PgDn`, `g/G`, `Esc` para cerrar.
- **Valor:** permite operar incidentes desde terminal sin perder contexto (hoy listas largas se truncan y el detalle es limitado).
- **Alcance sugerido:** `src/tui.rs` (estado de selección/scroll/overlay) y, si hace falta, consultas paginadas o acotadas en `src/db.rs`; reutilizar datos ya disponibles (`latest_target`, alertas, eventos).
- **Consideraciones:** mantener refresco eficiente (evitar recargas completas innecesarias), no bloquear la UI y conservar atajos actuales; e2e TUI en PTY cuando sea viable.

### N2. TUI: filtros, búsqueda y ordenación interactiva
- **Descripción:** filtros rápidos por vista (Targets: estado/check; Alertas: firing/resolved/severidad; Eventos: nivel/kind/origen), búsqueda textual (`/`) y ordenación (`o`) por campos relevantes (nombre, estado, latencia, recientes). Mostrar “X de Y” y limpieza de filtros (`Esc`).
- **Valor:** mejora drásticamente la usabilidad con muchos targets o historial de incidentes.
- **Alcance sugerido:** estado de filtros/orden en `src/tui.rs`; filtrado en memoria sobre conjuntos acotados o consultas parametrizadas en DB según volumen.
- **Consideraciones:** rendimiento y memoria acotada; feedback claro del filtro activo en cabecera/footer.

### N3. TUI: pausa de refresco, control de intervalo y accesibilidad/ayuda
- **Descripción:** `p` para pausar/reanudar auto-refresh, ajuste del intervalo de refresco de la TUI, overlay de ayuda (`?`) con atajos y significado de vistas, e indicador de “última actualización” y estado de colector/DB. Soporte de `NO_COLOR` y contraste/estilos legibles.
- **Valor:** evita que la pantalla “salte” mientras se inspecciona y mejora accesibilidad/descubribilidad.
- **Alcance sugerido:** `src/tui.rs` (loop de eventos/tick, render condicional y estilos), sin cambios de protocolo.
- **Consideraciones:** que la pausa no detenga el colector subyacente salvo decisión explícita; mantener compatibilidad de teclas existentes.

### N4. TUI: vista de capacidad (disco por filesystem) y detalle ampliado de target
- **Descripción:** en Sistema, panel/sección con uso por filesystem/mount (usado/total/%, peor caso) y errores recientes; en Targets, al seleccionar uno mostrar resumen ampliado (último check, latencia reciente como sparkline/texto, días de cert si aplica, y métricas numéricas de ventana cuando estén disponibles). Sin añadir gráficos históricos al dashboard.
- **Valor:** visibilidad operativa (capacidad y estado por servicio) desde terminal, útil en servidores sin navegador.
- **Alcance sugerido:** `src/tui.rs` y datos de `latest/history`/detalle de targets; coordinable con P1/P3 para reutilizar agregados por ventana.
- **Consideraciones:** presentación compacta en terminales pequeñas; cálculos acotados y datos best-effort según plataforma.

### N5. Etiquetas (labels) y agrupación de targets
- **Descripción:** soporte de `labels`/`tags` por target en configuración (p. ej. `env`, `service`, `region`) expuestos en API y usados para filtrar/ordenar/agrupar en dashboard (tabla) y TUI.
- **Valor:** organización y operación a escala (muchos endpoints) y base para reglas/silencios por grupo (complementa P2).
- **Alcance sugerido:** `src/config.rs` (parse/validación), modelo/DB si se persiste o solo config+runtime, `src/web/routes.rs` y UI/TUI.
- **Consideraciones:** compatibilidad con configs sin labels; normalizar claves/valores y evitar alta cardinalidad.

### N6. Prueba de notificaciones y diagnósticos de entrega
- **Descripción:** mecanismo de “test” de canales (CLI, endpoint admin y/o acción en UI) que envíe una notificación de prueba y reporte resultado por canal (éxito/error, latencia, código/response resumida), registrando el evento.
- **Valor:** reduce tiempo de diagnóstico de integraciones (webhook/Discord, Telegram, ntfy) y errores de configuración.
- **Alcance sugerido:** `src/notifications.rs`, `src/web/routes.rs` o CLI en `src/main.rs`, eventos en DB y UI mínima; respetar auth/permisos.
- **Consideraciones:** rate limiting y protección para no abusar de canales; no exponer secretos en respuestas/logs.

### N7. Configuración modular (includes) y validación enriquecida
- **Descripción:** permitir incluir targets/configuración desde archivos adicionales o directorio (glob) con recarga segura; añadir `--print-effective-config` (redactada) y validaciones con mensajes accionables (campo, valor, sugerencia) y detección de duplicados/conflictos.
- **Valor:** facilita gestionar muchos targets y reduce errores operativos de despliegue.
- **Alcance sugerido:** `src/config.rs` (carga/merge/validación), `src/main.rs` (flags), reload y documentación/ejemplos.
- **Consideraciones:** definir precedencia clara, evitar ciclos de include y mantener compatibilidad con `config.toml` único.

### N8. Reportes operativos exportables por ventana
- **Descripción:** generar resúmenes por rango (CSV/JSON/Markdown) con disponibilidad, conteos de incidentes/alertas, latencias agregadas y cambios de estado por target, orientados a postmortems/revisiones.
- **Valor:** entrega valor analítico sin añadir gráficos al dashboard y facilita compartir estado con terceros.
- **Alcance sugerido:** consultas agregadas en `src/db.rs`, endpoint de export/report en `src/web/routes.rs`, y acción de descarga en UI (y opcional en TUI).
- **Consideraciones:** acotar tamaño/tiempo de generación, respetar auth y coherencia con retención/rollups.

### N9. Diagnóstico de latencia por fases en checks HTTP
- **Descripción:** cuando sea factible, capturar y exponer tiempos por fase en checks HTTP (DNS, connect, TLS, TTFB/transfer) como campos numéricos y detalle legible en API/UI/TUI, manteniendo best-effort por plataforma/cliente.
- **Valor:** acelera el troubleshooting (distinguir fallo de red, TLS o aplicación) sin herramientas externas.
- **Alcance sugerido:** `src/collector/*` (instrumentación), modelo/DB para almacenar métricas opcionales, API/UI/TUI para visualizarlas.
- **Consideraciones:** costo de medición y compatibilidad del cliente HTTP/TLS; campos opcionales y nulos cuando no estén disponibles.

### N10. Retención diferenciada y presupuesto de recursos con observabilidad
- **Descripción:** controles para operar dentro de 5–50 MB: retención/límites diferenciados por tipo de dato (métricas, targets, alertas, eventos), muestreo o ajuste adaptativo bajo carga, y exposición de métricas internas (tamaño DB/WAL, filas, duración de colecciones/prunes, errores) en `/api/health` y `/metrics`.
- **Valor:** previsibilidad de consumo y estabilidad en VPS pequeños o con muchos targets.
- **Alcance sugerido:** `src/config.rs`, `src/db.rs` (políticas/prune/vacuum seguro y opcional), `src/collector.rs` y endpoints de estado/métricas; documentación de tuning.
- **Consideraciones:** defaults conservadores, operaciones de mantenimiento (vacuum/checkpoint) fuera de ruta crítica o bajo umbral, sin degradar datos necesarios para resúmenes.

## Orden sugerido (nuevas propuestas)

- **Corto plazo:** N5 (labels), N6 (test de notificaciones), N8 (reportes), N3/N1 (usabilidad TUI base).
- **Medio plazo:** N4 (capacidad/detalle), N2 (filtros/búsqueda TUI), N7 (config modular), N9 (diagnóstico HTTP), N10 (presupuesto/retención).
- **Siempre:** validar contra principios de bajo consumo, alcance fijo del dashboard y compatibilidad antes de implementar.
