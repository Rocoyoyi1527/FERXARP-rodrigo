# ACT-03: análisis SonarQube y cobertura

Análisis **real** del 26-09-2026 con SonarQube Community Build **26.9.0.129388** en contenedor local efímero `ferxarp_act03_sonar` (`http://127.0.0.1:19000`) y SonarScanner CLI **7.1.0.4889**. Dos proyectos: `ferxarp-frontend` (TypeScript/TSX, `frontend/src`) y `ferxarp-backend` (Rust, `backend/src`). Esta edición de SonarQube **sí analizó Rust**: el scanner cargó LCOV y encontró un code smell Rust, sin plugin externo. La cobertura Rust se generó con `cargo llvm-cov`; `cargo clippy` se ejecutó por separado y pasó sin warnings. No se enviaron `node_modules`, `.next`, `target`, `.env` ni volúmenes.

La puerta usada fue **Sonar way**, predeterminada sin modificar condiciones. En el primer análisis ambos proyectos dieron PASS sin condiciones de código nuevo; después del primer cambio el frontend dio FAIL porque `new_coverage=64.3% < 80%`. Se añadieron pruebas de `GardenGraph`, se repitió Jest y SonarScanner y el análisis final dio **PASS**: cobertura nueva frontend **93.1%**, duplicación nueva 0.0%, violaciones nuevas 0. Backend final PASS: cobertura nueva 100.0%, duplicación nueva 0.0%, violaciones nuevas 0. El gate mide código nuevo, por eso PASS no equivale a cobertura total >=80%.

## Métricas medidas

| Métrica | Frontend antes | Frontend después | Backend antes | Backend después |
|---|---:|---:|---:|---:|
| Quality Gate | PASS | PASS | PASS | PASS |
| Bugs | 0 | 0 | 0 | 0 |
| Vulnerabilities | 0 | 0 | 0 | 0 |
| Security hotspots | 0 | 0 | 0 | 0 |
| Code smells activos | 46 | 40 | 1 | 1 |
| Coverage Sonar, total | 68.8% | 77.8% | 78.1% | 77.9% |
| Duplicated lines density | 0.0% | 0.0% | 6.1% | 6.0% |
| Deuda técnica (`sqale_index`) | 291 min | 250 min | 6 min | 6 min |
| Maintainability rating | A (1.0) | A (1.0) | A (1.0) | A (1.0) |
| Reliability rating | A (1.0) | A (1.0) | A (1.0) | A (1.0) |
| Security rating | A (1.0) | A (1.0) | A (1.0) | A (1.0) |
| NLOC | 2204 | 2224 | 2755 | 2789 |

Fuentes sin credenciales: [`sonar-before-frontend.json`](sonar-before-frontend.json), [`sonar-after-frontend.json`](sonar-after-frontend.json), [`sonar-before-backend.json`](sonar-before-backend.json), [`sonar-after-backend.json`](sonar-after-backend.json). `issues_total` de la API puede incluir issues con `status: CLOSED`; la métrica `code_smells` cuenta los activos. El archivo final frontend conserva dos issues críticos S3776 como **CLOSED**. La deuda bajó 41 minutos y los smells activos bajaron 6; se extrajo la lógica de deduplicación y formato de `GardenGraph` sin cambiar la vista. El único issue crítico activo restante es `rust:S3776` en `backend/src/api/seed.rs:73` (complejidad 22 frente a 15), deuda estimada 6 min. El seed opcional no se refactorizó dentro de esta corrección de seguridad; queda como **P2**, sin ocultarlo.

Sonar no detectó el XSS real del popup Leaflet: estaba en la inserción dinámica de HTML y se verificó con Chromium, documentado en [seguridad](../security/README.md). Sus métricas de vulnerabilidades cero no sustituyen pruebas dinámicas. `frontend/coverage/lcov.info` proviene del conjunto Jest instrumentado; Jest final reportó 98.03% de líneas y 90.27% de ramas en ese conjunto, mientras Sonar incluye más archivos fuente y reporta 77.8% global. El módulo escolar Rust alcanzó 85.91% de líneas; Sonar informa 77.9% del backend completo. Son métricas de alcances distintos.

## Reproducción

```bash
docker run --detach --rm --name ferxarp_act03_sonar \
  --publish 127.0.0.1:19000:9000 \
  --env SONAR_ES_BOOTSTRAP_CHECKS_DISABLE=true sonarqube:community
# Espera /api/system/status = UP; crea un token de análisis local en la interfaz.
export SONAR_HOST_URL=http://127.0.0.1:19000
export SONAR_TOKEN='<token local; no guardarlo en el repositorio>'
export SONAR_SCANNER_BIN=/ruta/al/sonar-scanner-7.1.0.4889-linux-x64/bin/sonar-scanner
(cd frontend && pnpm test:coverage)
(cd backend && cargo clippy --all-targets --all-features -- -D warnings && ./scripts/coverage.sh)
scripts/sonar-scan.sh after
unset SONAR_TOKEN
docker stop ferxarp_act03_sonar
```

`backend/scripts/coverage.sh` requiere PostgreSQL/Chroma locales de prueba, `DATABASE_URL`, `CHROMA_URL`, `SQLX_OFFLINE=true`, y `LLVM_COV`/`LLVM_PROFDATA` si el toolchain necesita los binarios del sistema. `scripts/sonar-scan.sh` exige un token **solo en variable de entorno**, un scanner CLI explícito y ambos LCOV; falla si faltan. `scripts/sonar-metrics.py` consulta la API y guarda únicamente métricas, Quality Gate e issues, sin token. Se ejecutó el mismo comando para los snapshots inicial y final. Los snapshots iniciales son históricos y no se pueden recrear desde el código corregido sin cambiar de revisión.

SonarScanner advirtió una entrada no resuelta en el LCOV Rust, pero importó el reporte y calculó cobertura; la discrepancia entre su porcentaje y el resumen del módulo se debe también al alcance completo. El scanner avisó que archivos sin commit carecían de *blame* de Git; esto puede afectar atribución de código nuevo, y por ello se conservan los valores y condiciones exactos del Quality Gate en los JSON.
