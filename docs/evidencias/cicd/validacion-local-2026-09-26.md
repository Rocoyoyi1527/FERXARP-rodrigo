# ACT-02 — Evidencia de validación local (2026-09-26)

Rama `feat/activity-02-cicd`; base `b20fd21a7dbe3960efc3eb7fa85b54cc5a811246`. Se conservaron los cambios parciales encontrados al inicio. Todas las comprobaciones siguientes terminaron con código de salida 0, salvo el primer intento de cobertura sin herramientas LLVM configuradas; se corrigió el entorno local y la ejecución final pasó. No hubo commit ni push.

## Workflow y calidad

| Comprobación ejecutada | Resultado observado |
| --- | --- |
| `actionlint -color=false .github/workflows/ci-cd.yml` (v1.7.7), `bash -n` de scripts y `git diff --check` | Sin errores |
| `cargo fmt --check`, `SQLX_OFFLINE=true cargo check --locked`, `cargo clippy --all-targets --all-features -- -D warnings` | PASS |
| `cargo sqlx prepare --check -- --locked` contra PostgreSQL migrado, con SQLx CLI 0.9.0 | PASS; metadata actual |
| `cargo test --locked` contra PostgreSQL y ChromaDB aislados | 73/73 PASS: 10 lib, 52 main, 11 integración Chroma |
| `./scripts/coverage.sh` con cargo-llvm-cov 0.6.23 y LLVM 22.1.8 | PASS; 85.91% de líneas del módulo escolar, 74.48% de líneas Rust completas; [alcance definido en ACT-01](../testing/README.md) |
| `cargo build --release --locked --bins` con SQLx offline | PASS |
| `pnpm install --frozen-lockfile`, `pnpm lint`, `pnpm exec tsc --noEmit` | PASS |
| `pnpm test`, `pnpm test:coverage` | 35/35 PASS; 97.79% líneas e instrucciones, 93.18% funciones, 91.42% ramas; cuatro umbrales >=80% |
| `pnpm build` | PASS; Next.js 16.3.5, seis rutas estáticas más `/_not-found` |
| `docker compose config --quiet`, `docker compose build` y los dos comandos `docker build --tag ferxarp-{backend,frontend}:ci` del workflow | PASS; dos imágenes construidas |
| `docker build --no-cache` de ambas imágenes | PASS; instalación pnpm y compilación Rust/Next desde capas nuevas |

Las pruebas Rust locales usaron `SQLX_OFFLINE=true` para la compilación y variables descartables de CI para conectar a PostgreSQL y ChromaDB en puertos 15433 y 18001. El check de metadata SQLx sí consultó la base migrada. Este host Arch no tiene `rustup`; para cobertura se usaron `LLVM_COV=/usr/bin/llvm-cov` y `LLVM_PROFDATA=/usr/bin/llvm-profdata`. En GitHub Actions el workflow instala `llvm-tools-preview` mediante la acción de Rust.

Las mediciones quedan versionables en [rust-module-summary.txt](rust-module-summary.txt) y [frontend-coverage-total.json](frontend-coverage-total.json), generadas a partir de los reportes de esta ejecución.

## Deploy, readiness y smoke

Se ejecutó `POSTGRES_PORT=15434 CHROMA_PORT=18002 BACKEND_PORT=18000 FRONTEND_PORT=13000 ./scripts/deploy-test.sh`. El script construyó las imágenes para esos puertos, levantó PostgreSQL y ChromaDB saludables, ejecutó `/app/migrate`, provisionó el Admin de CI y esperó a backend y frontend saludables. La salida del smoke fue:

```text
ChromaDB: HTTP 200
Backend /health: HTTP 200
Frontend /: HTTP 200
Admin login: HTTP 200
Protected Admin route: HTTP 200
```

El script finalizó con código 0 y `docker compose down -v --remove-orphans` eliminó los cuatro contenedores, la red y los dos volúmenes de `ferxarp_ci_1308119`. El proyecto separado para las pruebas Rust, `ferxarp_act02_validation`, también se eliminó mediante `docker compose -p ferxarp_act02_validation down -v --remove-orphans`. La consulta posterior a `docker ps -a` y `docker volume ls` no mostró recursos con ninguno de esos dos prefijos. Los contenedores y volúmenes preexistentes del entorno local permanecieron intactos.

## Límites de la evidencia

Los builds Docker directos pasaron también sin caché local de capas. El daemon local usó el builder clásico porque `buildx` no está instalado; GitHub Actions usará su propio runner y falta observar esa ejecución real. No existen aún una ejecución remota de los cuatro jobs ni capturas de GitHub Actions: publicar estos cambios está fuera de ACT-02 por la instrucción de no hacer commit ni push. Los artefactos LCOV completos permanecen en `backend/coverage/` y `frontend/coverage/`, ignorados por Git; el workflow publicará resúmenes de ambos durante sus ejecuciones.
