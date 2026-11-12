# TinyTop - Monitor VPS Ligero en Rust

[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![size](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![platform](https://img.shields.io/badge/platform-linux-lightgrey.svg)](https://www.os-linux.org)

![TinyTop Dashboard](docs/dashboard.png)

## 📊 Monitor Autohospedado para VPS

**TinyTop** es una herramienta de monitoreo autohospedada y ligera para servidores y VPS. Recopila tráfico, latencia, CPU y memoria de aplicaciones configuradas y muestra gráficos simples en una interfaz dark mode moderna.

**Tamaño binario**: ~15-25MB estático  
**Consumo RAM idle**: 8-12MB (ideal para VPS con recursos limitados)  
**Arquitectura**: Statically linked binary, sin dependencias externas

---

## 🌟 Características

### Interfaz de Usuario
- **Dark mode** estilo Vercel con Tailwind CSS
- **Gráficos modernos** con Chart.js y gradientes dark mode
- **Grids responsivos**: CPU, RAM, Disk, Network
- **Actualización automática** cada 30 segundos
- **Historial 24h** con animaciones smooth
- **Responsive design** - funciona en mobile y desktop

### Monitoreo de Métricas
- **CPU**: Uso promedio de todos los procesadores
- **RAM**: Memoria disponible en MB
- **Disk**: Espacio disponible (lectura via sysinfo)
- **Network**: Bytes recibidos/transmitidos total
- **Targets personalizables**: Múltiples aplicaciones Java (Spring Boot, Quarkus, Micronaut)

### Integraciones de Aplicaciones
- **Spring Boot**: Actuator health y request metrics
- **Quarkus**: Request, latency, CPU vía Micrometer
- **Micronaut**: Request metrics vía Micrometer
- **Cualquier lenguaje**: Endpoint JSON personalizado

### Configuración
```toml
# config.toml
port = 9090
db_path = "tinytop.db"
interval_seconds = 30

[[targets]]
name = "localhost"
url = "http://localhost:8080/actuator/metrics"
kind = "spring-boot"

[[targets]]
name = "api"
url = "http://localhost:3000/api/health"
kind = "generic"
```

---

## 🚀 Despliegue Rápido

### Opción 1: Binario Rust Estático

```bash
# Clonar y compilar
git clone https://github.com/dev1lsconf/tinytop.git
cd tinytop
cargo build --release  # ~2-5 minutos

# Ejecutar
./target/release/tinytop
# Acceder: http://TU_IP:9090
```

### Opción 2: Docker (recomendado para VPS)

```bash
# Build y run con docker compose
git clone https://github.com/dev1lsconf/tinytop.git
cd tinytop
docker compose up -d  # Puerto 9090 expuesto

# Acceder: http://TU_IP:9090
# Ver logs: docker compose logs -f
```

### Opción 3: Script de instalación

```bash
curl -fsSL https://tu-dominio.com/install.sh | sh
```

---

## 📸 Captura de Pantalla

![TinyTop Dashboard](docs/dashboard.png)

*Dashboard dark mode con métricas de sistema (CPU/RAM/Disco/Red), gráficos históricos y estado de targets en tiempo real.*

### API Health
```bash
curl http://localhost:9090/api/health
# {"status":"ok","version":"0.1.0","targets":1}

curl http://localhost:9090/api/metrics/latest
# [{"type":"system","cpu":45.2,"memory_mb":384.3},{"type":"target","name":"localhost","status":"up"}]
```

---

## 🛠️ Arquitectura Técnica

### Stack Tecnológico

| Capa | Tecnología | Por qué |
|------|-----------|---------|
| **Runtime** | Rust 1.85 + Tokio | Sin GC, bajo overhead, binario estático |
| **HTTP Server** | Axum | Moderno, type-safe, minimal |
| **SQLite** | sqlx + rusqlite | Embedded, sin proceso separado |
| **Métricas sistema** | sysinfo | Bajo overhead, multiplataforma |
| **Frontend** | HTML + Tailwind CDN + Chart.js | Sin build step, sirve desde binario |
| **HTTP Client** | reqwest | Fetch a aplicaciones Java |

### Arquitectura de Sistema

```
┌─────────────────────────────────────────┐
│             TinyTop Binary               │
│  ┌─────────────┐  ┌─────────────────┐ │
│  │ Axum Server │  │ tokio Runtime   │ │
│  └──────┬──────┘  └───────┬─────────┘ │
│          │                      │      │
│   API REST│                      │ Background Collector (cada 30s)
│          │                      │
│          ▼                      ▼
│  ┌─────────────────────────────────────┐ │
│  │ SQLite Database (tinytop.db)         │ │
│  └─────────────────────────────────────┘ │
│                ▲                        │
│                │ Fetch from           │
│      Java Apps (Spring/Quarkus/Micronaut)│
└─────────────────────────────────────────┘
```

---

## 📡 API Endpoints

| Endpoint | Método | Descripción |
|----------|--------|-------------|
| `/` | GET | Página HTML con dashboard |
| `/api/health` | GET | Status del servicio + versión |
| `/api/metrics/latest` | GET | Métricas actuales + status targets |
| `/api/metrics/history` | GET | Histórico con `?range=24&target=name` |
| `/api/config` | GET | Configuración actual de targets |

### Ejemplos de Respuesta

```bash
# Health check
curl http://localhost:9090/api/health
{"status":"ok","version":"0.1.0","targets":1}

# Latest metrics
curl http://localhost:9090/api/metrics/latest
[{"type":"system","cpu":45.2,"memory_mb":384.3,"disk_mb":8192.0},{"type":"target","name":"localhost","status":"up","latency_ms":12.5}]

# History 24h
curl "http://localhost:9090/api/metrics/history?range=24&target=localhost"
[{"id":1,"target":"localhost","timestamp":"2026-01-15T10:30:00Z","cpu":42.1,"memory_mb":500.0,"disk_mb":8000.0,"network_rx":1048576,"network_tx":524288,"latency_ms":15.0,"http_requests":128}]
```

---

## 📦 Variables de Entorno

| Variable | Predeterminado | Descripción |
|----------|----------------|-------------|
| `TINYTOP_PORT` | `9090` | Puerto del servidor HTTP |
| `TINYTOP_DB_PATH` | `tinytop.db` | Ruta archivo SQLite |
| `TINYTOP_INTERVAL` | `30` | Intervalo recolección métricas (segundos) |
| `TINYTOP_CONFIG` | `config.toml` | Archivo de configuración |

```bash
# Ejecutar con vars de entorno
TINYTOP_PORT=3000 TINYTOP_DB_PATH=/var/lib/tinytop/tinytop.db ./target/release/tinytop
```

---

## 🐳 Despliegue con Docker

```dockerfile
# Usa el Dockerfile incluido
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates
COPY --from=builder /app/target/release/tinytop /usr/local/bin/tinytop
EXPOSE 9090
ENTRYPOINT ["tinytop"]
```

```bash
docker build -t tinytop .
docker run -d -p 9090:9090 --name tinytop tinytop
```

Con docker-compose:
```yaml
version: '3.8'
services:
  tinytop:
    build: .
    ports:
      - "9090:9090"
    volumes:
      - ./config.toml:/app/config.toml
      - ./tinytop.db:/app/tinytop.db
    restart: unless-stopped
```

---

## 📜 Licencia

**MIT** - Copyright (c) 2026 TinyTop Contributors

---

## 🙏 Agradecimientos

- [Rust Language](https://www.rust-lang.org) - Lenguaje de programación
- [Axum](https://github.com/tokio-rs/axum) - Framework HTTP
- [Chart.js](https://chartjs.org) - Gráficos visuales
- [Tailwind CSS](https://tailwindcss.com) - Styling utility-first

---

## 📬 Contacto

- **GitHub**: [https://github.com/dev1lsconf/tinytop](https://github.com/dev1lsconf/tinytop)
- **Issues**: [https://github.com/dev1lsconf/tinytop/issues](https://github.com/dev1lsconf/tinytop/issues)
- **Discussions**: [https://github.com/dev1lsconf/tinytop/discussions](https://github.com/dev1lsconf/tinytop/discussions)