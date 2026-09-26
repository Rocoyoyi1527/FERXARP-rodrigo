# ACT-02 — CI/CD de FERXARP

Fecha: 2026-09-26. Workflow: [`.github/workflows/ci-cd.yml`](../../../.github/workflows/ci-cd.yml), nombre visible **FERXARP CI/CD**. El pre-flight se hizo en `feat/activity-02-cicd`, commit base `b20fd21a7dbe3960efc3eb7fa85b54cc5a811246`, con árbol inicial limpio.

## Triggers y secuencia

`pull_request` y `workflow_dispatch` ejecutan pruebas y builds; `push` a `main` ejecuta además `deploy-test-environment`. El despliegue **no** corre en pull requests. Los jobs de calidad pueden avanzar en paralelo; `docker-build` espera a ambos, y el despliegue espera a los tres. `permissions: contents: read` limita el token del workflow y cada checkout usa `persist-credentials: false`.

| Etapa | Acción | Resultado esperado |
| --- | --- | --- |
| `backend-quality` | PostgreSQL 16 y ChromaDB 1.5.0 locales; migraciones, fmt, check, SQLx prepare, 73 tests Rust, Clippy, cobertura y release build de binarios | PASS; módulo Rust >=80% líneas; artefactos LCOV y resúmenes |
| `frontend-quality` | Node 22, pnpm 11.26.0; instalación congelada, lint, tsc, 35 tests Jest, cobertura y build Next | PASS; cuatro umbrales Jest >=80%; resumen JSON |
| `docker-build` | Construye las imágenes de backend y frontend desde los Dockerfiles versionados | Ambas imágenes construidas |
| `deploy-test-environment` | Solo push a `main`: Compose efímero, migración, Admin de CI, readiness y smoke HTTP | Cuatro servicios saludables; smoke PASS; `down -v` al salir |

Los tests Jest ya ejecutan `--runInBand` mediante el script `pnpm test`; añadir `pnpm test -- --runInBand` con esta versión de pnpm introduciría un `--` literal que Jest interpreta como patrón. El job ejecuta `pnpm test`, equivalente y validado.

## Despliegue de prueba

[`scripts/deploy-test.sh`](../../../scripts/deploy-test.sh) crea un proyecto Compose propio y un archivo de entorno temporal fuera del repositorio. Construye imágenes en su runner, espera los healthchecks de PostgreSQL y Chroma, aplica las migraciones mediante `/app/migrate`, y crea el Admin de CI con `/app/provision_admin`. Luego espera backend y frontend saludables. [`scripts/smoke-test.sh`](../../../scripts/smoke-test.sh) exige HTTP 200 en Chroma, `/health`, la portada, login Admin y `GET /api/auth/ngos` con JWT Admin. El token y la contraseña no se imprimen. Al salir, también si hay error, guarda hasta 100 líneas de logs por servicio para diagnóstico y ejecuta `docker compose down -v --remove-orphans` sobre **ese proyecto efímero**. El job falla si build, migración, provisión, readiness, smoke o limpieza fallan.

El job de despliegue reconstruye las imágenes porque GitHub Actions le asigna un runner distinto al de `docker-build`; no depende de un registro de imágenes ni de un servidor externo. Esto demuestra un **deployment automatizado de prueba**, no un despliegue de producción.

Los valores `ci-only-*` son descartables y públicos para este runner aislado. No hay secretos reales, `GROQ_API_KEY` queda ausente, el archivo `.env` local no se reemplaza y los `.dockerignore` impiden incluir `.env*` en las imágenes. No se usa `pull_request_target`, `continue-on-error`, ni se suben `node_modules`, `target`, bases de datos o volúmenes.

## Resultado local del 2026-09-26

| Comprobación | Resultado |
| --- | --- |
| `actionlint` y sintaxis YAML/Bash | PASS |
| `cargo fmt --check`, `cargo check --locked`, `cargo test --locked`, Clippy, `cargo sqlx prepare --check` | PASS; 73 tests Rust |
| `./scripts/coverage.sh` | PASS; módulo escolar 85.91% líneas; reporte Rust completo 74.48% líneas |
| `cargo build --release --locked --bins` | PASS |
| `pnpm lint`, `pnpm exec tsc --noEmit`, `pnpm test`, `pnpm test:coverage`, `pnpm build` | PASS; 35 tests Jest, 97.79% líneas, 91.42% ramas |
| `docker compose config --quiet`, `docker compose build`, builds Docker directos con y sin caché | PASS para ambas imágenes |
| `./scripts/deploy-test.sh` con puertos alternativos | PASS; cuatro servicios saludables, cinco comprobaciones HTTP 200, contenedores y volúmenes eliminados |

El detalle reproducible de esta ejecución y la comprobación posterior de limpieza están en [validacion-local-2026-09-26.md](validacion-local-2026-09-26.md). El umbral Rust de 80% se aplica al módulo escolar definido en ACT-01; el reporte completo y el porcentaje de funciones quedan visibles y no cumplen ese umbral. La comprobación local ejercitó las mismas órdenes principales del workflow; una ejecución real de GitHub Actions queda pendiente hasta que el equipo publique los cambios.

## Evidencia que debe capturarse en GitHub Actions

Después de publicar esta rama mediante el proceso normal del equipo, abrir la ejecución **FERXARP CI/CD** para un PR y para un push a `main`. Capturar: (1) los tres jobs de pruebas/build en verde en el PR, con deploy omitido; (2) los cuatro jobs en verde en `main`; (3) las líneas de salida de cobertura Rust/Jest; (4) el smoke de Chroma, backend, frontend, login y ruta Admin; (5) los artefactos `rust-coverage` y `jest-coverage-summary`. Si deploy falla, descargar `deploy-test-logs` y conservar el error del job. No se ha ejecutado todavía este workflow en GitHub porque ACT-02 prohíbe commit y push.

## Reproducción local

Desde la raíz, con Docker disponible y `backend/.env` local para los dos comandos Compose manuales. El script crea su propio archivo temporal y puede ejecutarse sin ese archivo:

```bash
docker compose config --quiet
docker compose build
POSTGRES_PORT=15433 CHROMA_PORT=18001 BACKEND_PORT=18000 FRONTEND_PORT=13000 ./scripts/deploy-test.sh
```

Los puertos alternativos permiten conservar el Compose de desarrollo existente. El script limpia su proyecto y sus volúmenes. La validación individual se reproduce con los comandos de [Pruebas y cobertura](../../../README.md#pruebas-y-cobertura), más `cd backend && cargo build --release --locked --bins` y `cd frontend && pnpm build`.

## Preparación para ACT-03, sin ejecutarla

Para dejar la aplicación levantada durante un escaneo posterior, usar el Compose de desarrollo persistente, no el script efímero:

```bash
docker compose build
docker compose up -d --wait --wait-timeout 180 postgres chromadb
docker compose run --rm --no-deps backend /app/migrate
docker compose up -d --wait --wait-timeout 180 backend frontend
```

Con `backend/.env` local configurado, las URL son frontend `http://localhost:3000`, backend `http://localhost:8000` y API health `http://localhost:8000/health`. Para rutas Admin, provisionar con `/app/provision_admin` pasando `FERXARP_ADMIN_EMAIL` y `FERXARP_ADMIN_PASSWORD` locales mediante `docker compose run --rm --no-deps -e FERXARP_ADMIN_EMAIL -e FERXARP_ADMIN_PASSWORD backend /app/provision_admin`; después hacer login en `POST /api/auth/login` y enviar el JWT en `Authorization: Bearer ...`. Rutas para una futura revisión ZAP: `/api/auth/register`, `/api/auth/login`, `/api/auth/me`, `/api/auth/ngos`, `/api/donations`, `/api/donations/feed`, `/api/donations/shipments` y `/api/scanner/scan`. Las rutas protegidas requieren un token del rol adecuado; las mutaciones deben usar datos descartables.

Para el análisis Sonar posterior: levantar SonarQube Community Build local, generar los LCOV con `cd backend && ./scripts/coverage.sh` y `cd frontend && pnpm test:coverage`, instalar SonarScanner CLI, crear un token de análisis en SonarQube y ejecutar el escáner en cada directorio. Jest escribe rutas de fuente relativas a `frontend`, por eso cada proyecto usa su propia raíz:

```bash
docker run -d --name ferxarp-sonarqube -p 9000:9000 sonarqube:community
export SONAR_HOST_URL=http://localhost:9000
read -r -s -p 'Sonar token: ' SONAR_TOKEN
printf '\n'
export SONAR_TOKEN
(cd backend && sonar-scanner -Dsonar.projectKey=ferxarp-backend -Dsonar.sources=src -Dsonar.rust.lcov.reportPaths=coverage/lcov.info)
(cd frontend && sonar-scanner -Dsonar.projectKey=ferxarp-frontend -Dsonar.sources=src -Dsonar.javascript.lcov.reportPaths=coverage/lcov.info)
unset SONAR_TOKEN
```

Estos parámetros de importación LCOV para Rust y TypeScript siguen la [documentación de SonarSource](https://docs.sonarsource.com/sonarqube-server/analyzing-source-code/test-coverage/test-coverage-parameters). El análisis ZAP/Sonar no forma parte de ACT-02 y no se ejecutó.
