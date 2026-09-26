# Evidencia remota GitHub Actions

Repositorio evaluado: [`Rocoyoyi1527/FERXARP-rodrigo`](https://github.com/Rocoyoyi1527/FERXARP-rodrigo). Workflow: **FERXARP CI/CD**. Se consultaron los runs y artefactos mediante GitHub CLI; los resultados siguientes corresponden al runner remoto, separados de la [validación local de ACT-02](validacion-local-2026-09-26.md).

## Pull Request

- **PR:** [#1](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/pull/1).
- **Run ID:** [`36277313303`](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36277313303).
- **Evento:** `pull_request`; rama `docs/activity-05-continuous-improvement`.
- **Resultado del workflow:** `success` (**PASS**).

| Job | Resultado remoto |
|---|---|
| Backend tests, coverage and release build | PASS |
| Frontend tests, coverage and build | PASS |
| Build backend and frontend Docker images | PASS |
| Deploy ephemeral test environment and smoke test | **Omitido correctamente**: el workflow lo condiciona a `push` en `main` |

Artefactos generados y consultados en el run: `jest-coverage-summary` y `rust-coverage` (ambos disponibles al verificar esta evidencia). La subida de cada artefacto figura también como paso exitoso de su job.

## Main

- **Rama:** `main`; **evento:** `push`.
- **Run ID:** [`36278602691`](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36278602691).
- **Resultado del workflow:** `success` (**PASS**).

| Job | Resultado remoto |
|---|---|
| Backend tests, coverage and release build | PASS |
| Frontend tests, coverage and build | PASS |
| Build backend and frontend Docker images | PASS |
| Deploy ephemeral test environment and smoke test | PASS |

También se consultaron en `main` los artefactos `rust-coverage` y `jest-coverage-summary`.

## Deploy test

El job remoto de `main` ejecutó el paso **Deploy, migrate, provision, smoke and clean up** con resultado `success`. El [script del workflow](../../../scripts/deploy-test.sh) creó un proyecto Compose efímero con PostgreSQL, ChromaDB, backend y frontend; construyó imágenes, aplicó migraciones, provisionó el Admin de CI y esperó la disponibilidad de los servicios. El log remoto registró los cinco smoke checks de [`scripts/smoke-test.sh`](../../../scripts/smoke-test.sh):

| Comprobación remota | Resultado en log |
|---|---|
| ChromaDB | HTTP 200 |
| Backend `/health` | HTTP 200 |
| Frontend `/` | HTTP 200 |
| Login Admin CI | HTTP 200 |
| Ruta Admin protegida | HTTP 200 |

El mismo log registró la eliminación de los contenedores y de los volúmenes `postgres_data` y `chroma_data`. El script ejecuta `docker compose down -v --remove-orphans` al salir. Esta evidencia acredita un **despliegue efímero de prueba**, no un despliegue de producción.

Para repetir la consulta de solo lectura: `gh run view 36277313303 --repo Rocoyoyi1527/FERXARP-rodrigo`, `gh run view 36278602691 --repo Rocoyoyi1527/FERXARP-rodrigo` y `gh api repos/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36277313303/artifacts` (sustituyendo el ID para `main`). La disponibilidad de artefactos puede cambiar por su retención; los enlaces de los runs son la fuente primaria.

## Conclusión

- TEST = PASS
- BUILD = PASS
- DEPLOY = PASS
