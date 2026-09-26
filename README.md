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
  * Control logístico en 4 etapas (`/shipments`): *Solicitadas*, *Listas para salida*, *En camino* y *Finalizadas*.
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

4. Desde `backend/`, ejecuta `cargo check --locked`, `cargo run --locked` y `cargo test --locked`. Verifica `http://localhost:8000/health` cuando el backend esté en ejecución. Los tests de autorización crean automáticamente la base local `ferxarp_security_test` y aplican sus migraciones; el usuario de PostgreSQL debe poder crear bases de datos.

El seed de demostración requiere un JWT de Admin y `FERXARP_SEED_PASSWORD` en `backend/.env`. Configura una contraseña local propia antes de invocarlo; las ONG sembradas empiezan sin verificar. Para poblar e indexar sin enriquecimiento Groq, usa `POST /api/seed/veracruz?include_ai=false`.

## ChromaDB y matching local

Levanta PostgreSQL y ChromaDB con `docker compose up -d postgres chromadb`; espera a que ambos aparezcan como `healthy` en `docker compose ps`. Para ejecutar el backend fuera de Docker, usa `CHROMA_URL=http://localhost:8001` en `backend/.env`. Dentro de Compose se usa `http://chromadb:8000`. ChromaDB 1.5.0 persiste su índice en un volumen Docker; PostgreSQL conserva los perfiles y necesidades originales.

Desde `backend/`, ejecuta `cargo run --locked --bin reindex_chroma` para reconstruir la colección `ngo_needs` desde todas las ONG de PostgreSQL. El comando informa cuántas procesó, indexó y falló; puede repetirse sin duplicar documentos. Si encuentra esa colección derivada con una métrica anterior incompatible, la reemplaza y la reconstruye desde PostgreSQL. El registro de ONG y el seed protegido usan el mismo servicio de indexación. Si Chroma falla durante un registro, la cuenta permanece en PostgreSQL y el log indica que debe reindexarse. Los tests de integración requieren Chroma y PostgreSQL locales levantados.

`GET /api/donations/{id}/matches?include_ai=false` comprueba el matching sin Groq. La cabecera `X-Matching-Mode` indica `hybrid` cuando Chroma respondió o `lexical_fallback` cuando falta, falla o agota el tiempo de espera. La respuesta incluye `lexical_score`, `vector_score` y `semantic_similarity` (su combinación); la consulta vectorial se limita a los UUID de ONG presentes en PostgreSQL. Si no hay coincidencia de contenido, la lista queda vacía. Los vectores son rasgos locales deterministas con sinónimos de dominio: permiten búsqueda coseno reproducible sin descargar un modelo; su alcance semántico es limitado. Chroma usa distancia coseno; `vector_score = clamp(1 - distancia, 0, 1)`. Si Chroma cae, login, donaciones y envíos siguen funcionando y el matching usa solo la parte léxica.

## Provisionar administrador inicial

Con PostgreSQL local levantado y las migraciones aplicadas, entra en `backend/` y define `FERXARP_ADMIN_EMAIL` y `FERXARP_ADMIN_PASSWORD`. La contraseña debe tener al menos 12 caracteres. Para introducirla sin mostrarla ni incluirla en el historial de comandos:

```bash
export FERXARP_ADMIN_EMAIL='<correo del administrador>'
read -r -s -p 'Contraseña inicial: ' FERXARP_ADMIN_PASSWORD
printf '\n'
export FERXARP_ADMIN_PASSWORD
cargo run --locked --bin provision_admin
unset FERXARP_ADMIN_EMAIL FERXARP_ADMIN_PASSWORD
```

Repetir el comando para el mismo Admin no cambia su contraseña. Si el correo pertenece a otro rol, el comando rechaza la elevación. `POST /api/auth/register` solo admite `empresa` y `ong`; no crea administradores.

SQLx CLI y las macros `query!` leen `backend/.env` en el directorio de trabajo. La imagen Docker usa metadata versionada en `.sqlx` para compilar sin incluir secretos.

Para usar Compose completo, `docker compose up --build` toma `backend/.env` en tiempo de ejecución y conecta el backend al servicio `postgres`. La imagen no contiene el archivo `.env`.

## Frontend local

Desde `frontend/`, usa pnpm y el lockfile versionado:

```bash
pnpm install --frozen-lockfile
pnpm dev
```

Con el backend local disponible en `http://localhost:8000`, abre `http://localhost:3000`. Para validar el frontend ejecuta `pnpm lint`, `pnpm exec tsc --noEmit` y `pnpm build`. La aprobación de una solicitud mantiene la donación reservada; la Empresa registra la salida desde `/shipments` y la ONG registra la entrega o el rechazo con motivo.

## Pruebas y cobertura

Levanta PostgreSQL y ChromaDB locales (`docker compose up -d postgres chromadb`) y aplica las migraciones (`cd backend && sqlx migrate run`) antes de probar Rust. Los tests crean una base aislada `ferxarp_security_test`. Instala `cargo-llvm-cov` y los componentes LLVM de tu toolchain; en distribuciones que usan LLVM del sistema, exporta `LLVM_COV=/usr/bin/llvm-cov` y `LLVM_PROFDATA=/usr/bin/llvm-profdata`. SQLx puede usar la metadata versionada con `SQLX_OFFLINE=true` durante tests y cobertura.

```bash
cd backend
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo sqlx prepare --check -- --locked
mkdir -p coverage
cargo llvm-cov --all-features --workspace --lcov --output-path coverage/lcov.info
./scripts/coverage.sh
```

`scripts/coverage.sh` genera resúmenes completos y del módulo escolar y falla si el segundo baja de 80% de líneas. El segundo incluye JWT, RBAC, donaciones, escáner, bootstrap Admin y matching local/híbrido; excluye Groq, métricas CEO, seed de demostración, binarios CLI, arranque y código de la biblioteca estándar. El LCOV completo y ambos resúmenes quedan en `backend/coverage/` (ignorado por Git). La cobertura del módulo supera 80% de líneas; el total completo se informa también en [la evidencia de pruebas](docs/evidencias/testing/README.md).

```bash
cd frontend
pnpm install --frozen-lockfile
pnpm lint
pnpm exec tsc --noEmit
pnpm test
pnpm test:coverage
pnpm build
```

Jest mide de forma fija `src/lib/api.ts`, las páginas de login, registro y envíos, y `StockScanner.tsx`, con umbral global de 80% para líneas, instrucciones, funciones y ramas. `frontend/coverage/` se genera localmente y está ignorado por Git.

## CI/CD

El workflow [FERXARP CI/CD](.github/workflows/ci-cd.yml) valida Rust y Next.js en pull requests y pushes a `main`; después construye ambas imágenes Docker. Un push a `main` despliega además una instancia de prueba efímera en el runner, aplica migraciones, provisiona un Admin exclusivo de CI, comprueba Chroma, backend, frontend, login y una ruta Admin, y elimina contenedores y volúmenes al terminar. No despliega producción ni requiere credenciales reales. Los umbrales de cobertura de ACT-01 rompen el job si retroceden.

Para repetir el deploy de prueba sin tocar los contenedores locales existentes, desde la raíz usa puertos alternativos. Los comandos Compose manuales requieren `backend/.env` local; el script crea su propio archivo temporal y puede ejecutarse sin él:

```bash
docker compose config --quiet
docker compose build
POSTGRES_PORT=15433 CHROMA_PORT=18001 BACKEND_PORT=18000 FRONTEND_PORT=13000 ./scripts/deploy-test.sh
```

El script usa solo valores CI descartables, un proyecto Compose propio y un archivo temporal fuera del repositorio; borra sus volúmenes al salir. La [evidencia del pipeline](docs/evidencias/cicd/README.md) enumera jobs, artefactos y pasos para revisar la ejecución en GitHub Actions.

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
│   │   │   └── shipments/    # Control de solicitudes y envíos en 4 etapas
│   │   ├── components/       # Componentes visuales (GardenGraph, Navbar, etc.)
│   │   └── lib/api.ts        # Cliente tipado de consumo backend
│   └── package.json
├── documentacion.md          # Documento formal del sistema
└── README.md
