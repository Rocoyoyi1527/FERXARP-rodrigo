# 🌿 Fexarp — Ecosistema de Distribución Solidaria de Excedentes

Plataforma integral orientada a canalizar excedentes operativos y alimentarios de empresas hacia organizaciones sociales verificadas mediante emparejamiento semántico vectorial, geolocalización en tiempo real y trazabilidad física con bitácora inmutable.

---

## 🚀 Estado Actual del Proyecto (Fase 7 Concluida)

* **Backend (Rust / Axum):** 
  * Monolito modular con control de acceso RBAC (`Empresa`, `Ong`, `Ceo`, `Admin`).
  * Motor de emparejamiento híbrido: similitud vectorial (ChromaDB) + fórmula geodésica (Haversine) + análisis cognitivo vía Groq AI (`deepseek-r1-distill-llama-70b`).
  * Bitácora de transiciones y rechazos de stock (`scanner.rs` sobre tabla `delivery_logs`).
  * Módulo de ingesta y siembra local idempotente (`seed.rs`).
* **Frontend (Next.js / Tailwind CSS):**
  * Interfaz con estética botánica mineral e iconografía intuitiva sin jerga técnica.
  * Tableros diferenciados por rol:
    * **Empresa:** Formulario de registro de lotes y grafo interactivo de compatibilidad (`GardenGraph.tsx`).
    * **ONG:** Dosel de absorción de lotes disponibles, apartado con un clic y escáner de recepción.
    * **CEO:** Métricas de sostenibilidad, volumen transferido, beneficiarios directos y $\text{CO}_2\text{e}$ evitado.
    * **Admin TI:** Auditoría y verificación oficial de organizaciones para evitar riesgos de corrupción.
  * Embudo de control logístico en 3 etapas (`/shipments`): *1. Solicitadas*, *2. En Camino*, *3. Finalizadas*.
  * Mapa geoespacial interactivo (`/map`) adaptado con teselas botánicas y rutas de acopio.

---

## 🛠️ Stack Tecnológico

| Capa | Tecnologías |
| :--- | :--- |
| **Backend** | Rust 2021, Axum, Tokio, SQLx (PostgreSQL), Jsonwebtoken, Reqwest |
| **Frontend** | Next.js (App Router), React, Tailwind CSS, Leaflet |
| **Bases de Datos** | Supabase (PostgreSQL relacional) + ChromaDB (Base vectorial) |
| **Inteligencia Artificial** | Groq LPU (DeepSeek-R1 Distill Llama 70B) + Embeddings Cosine |

## Backend local desde un checkout limpio

1. Copia `backend/.env.example` a `backend/.env` y reemplaza `JWT_SECRET` por un secreto local. El ejemplo de `DATABASE_URL` usa el puerto local `5433`; PostgreSQL escucha en `5432` dentro de Docker.
2. Levanta la base local: `docker compose up -d postgres`. Espera hasta que `docker compose ps postgres` indique `healthy`.
3. Desde `backend/`, aplica las migraciones con `sqlx migrate run`. Si no tienes SQLx CLI, instálalo solamente en una ruta local y ejecútalo desde allí:

   ```bash
   cargo install sqlx-cli --version 0.9.0 --locked --no-default-features --features postgres,rustls --root /tmp/ferxarp-sqlx-tools
   /tmp/ferxarp-sqlx-tools/bin/sqlx migrate run
   ```

4. Desde `backend/`, ejecuta `cargo check --locked`, `cargo run --locked` y `cargo test --locked`. Verifica `http://localhost:8000/health` cuando el backend esté en ejecución.

SQLx CLI y las macros `query!` leen `backend/.env` en el directorio de trabajo. La imagen Docker usa metadata versionada en `.sqlx` para compilar sin incluir secretos.

Para usar Compose completo, `docker compose up --build` toma `backend/.env` en tiempo de ejecución y conecta el backend al servicio `postgres`. La imagen no contiene el archivo `.env`.

---

## 📁 Estructura del Monorepositorio

```text
FERXARP/
├── backend/                  # Dominio en Rust (Axum + SQLx)
│   ├── src/
│   │   ├── ai/               # ChromaDB client, scorer, matcher y Groq AI
│   │   │   ├── chroma_db.rs
│   │   │   ├── groq.rs
│   │   │   ├── matcher.rs
│   │   │   └── scoring.rs
│   │   ├── api/              # Endpoints HTTP REST
│   │   │   ├── auth.rs       # RBAC, login/registro, verificación ONGs
│   │   │   ├── donations.rs  # CRUD, matching, solicitudes y envíos
│   │   │   ├── metrics.rs    # Agregaciones e impacto para el CEO
│   │   │   ├── scanner.rs    # Salidas, recepciones y rechazos de stock
│   │   │   └── seed.rs       # Ingesta idempotente de Veracruz y Groq test
│   │   ├── models/           # Structs de dominio (User, Ngo, Roles)
│   │   └── main.rs           # Configuración del servidor y ruteo central
│   └── Cargo.toml
├── frontend/                 # Aplicación Next.js (App Router)
│   ├── src/
│   │   ├── app/
│   │   │   ├── (auth)/       # Vistas de autenticación (Login / Registro)
│   │   │   ├── dashboard/    # Tablero dinámico adaptado según el rol
│   │   │   ├── map/          # Topografía logística sobre Leaflet
│   │   │   └── shipments/    # Embudo de envíos y solicitudes en 3 etapas
│   │   ├── components/       # Componentes visuales (GardenGraph, Navbar, etc.)
│   │   └── lib/api.ts        # Cliente tipado de consumo backend
│   └── package.json
├── documentacion.md          # Documento formal del sistema
└── README.md
