# TinyTop - Convención de Commits con Emoji

## Formato
```
<emoji>(categoria opcional): descripción corta

[pie de página opcional]
```

## Emojis por Tipo

| Emoji | Tipo | Descripción |
|-------|------|-------------|
| ✨ | Features | Nueva funcionalidad |
| 🐛 | Bug Fix | Corrección de bugs |
| 📦 | Docs | Actualizaciones de documentación |
| ♻️ | Refactor | Refactoring de código |
| 🎨 | UI | Cambios en la interfaz de usuario |
| ♿ | Accessibility | Mejoras de accesibilidad |
| 🔧 | Config | Cambios de configuración |
| 🧪 | Test | Añadiendo tests |
| 🚀 | Performance | Mejoras de rendimiento |
| 📈 | Metrics | Nuevas métricas |
| 🗑️ | Delete | Eliminando código |

## Categorías por Área

| Categoría | Prefijo |
|-----------|---------|
| Frontend | `fe:` |
| Backend/API | `be:` |
| Configuración | `config:` |
| Base de datos | `db:` |
| Dependencies | `deps:` |
| Docker | `docker:` |
| Testing | `test:` |

## Ejemplos

```bash
✨ feat(api): añadir endpoint /api/metrics/latest
♻️ refactor(collector): mejorar lógica de colección
🐛 fix(api): corregir parámetro opcional en history
📦 docs(readme): actualizar instrucciones de despliegue
🎨 ui(index): mejorar dark mode styling
🔧 config: actualizar intervalos de recolección
📈 metrics: añadir métricas de disco
```

## Mensajes de Commits Requeridos

1. **Tipo y emoji obligatorios** al inicio
2. **Cuerpo opcional** (máx 200 caracteres)
3. **Footer opcional** para issues o breaking changes
4. **No capitalizar** el emoji (forma parte del mensaje)

## Breaking Changes

Cuando hagas un cambio que rompe la compatibilidad anteriror, añade `!` después del emoji:

```bash
✨ feat!: refactor(api): restructurar endpoints REST
```

o en el footer:

```
BREAKING CHANGE: endpoint /api/metrics ahora requiere parámetro target
```

## Flujo de Trabajo Recomendado

1. `git checkout -b feature/nueva-funcionalidad`
2. `git add .`
3. `git commit -m "✨ feat: descripción del cambio"`
4. `git push origin feature/nueva-funcionalidad`
5. Crear Pull Request