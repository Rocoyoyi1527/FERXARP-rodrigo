# Matriz de cumplimiento de rúbrica FERXARP

**Corte de auditoría original:** 2026-09-26, rama `docs/activity-05-continuous-improvement`, HEAD `d0cf3478a5da9403a7085ece235c2ee10fe56434`. **Actualización posterior:** se incorporó la [evidencia remota del repositorio `Rocoyoyi1527/FERXARP-rodrigo`](../evidencias/cicd/github-actions-remote.md), verificada por GitHub CLI. El árbol estaba limpio antes de esta actualización; **no se repitieron** suites, builds ni escáneres. Los estados describen evidencia disponible, no una calificación.

Fuentes: [README](../../README.md), [documentación inicial](../../documentacion.md), [testing](../evidencias/testing/README.md), [CI/CD](../evidencias/cicd/README.md), [seguridad](../evidencias/security/README.md), [calidad](../evidencias/quality/README.md), [informe de cierre](informe-cierre.md), [plan de mejora](plan-mejora-continua.md), [workflow](../../.github/workflows/ci-cd.yml), [`backend/scripts/coverage.sh`](../../backend/scripts/coverage.sh), [`scripts/deploy-test.sh`](../../scripts/deploy-test.sh), [`scripts/smoke-test.sh`](../../scripts/smoke-test.sh), [`scripts/security-scan.sh`](../../scripts/security-scan.sh) y [`scripts/sonar-scan.sh`](../../scripts/sonar-scan.sh).

## 1. Implementación y seguridad

El módulo funcional tiene backend Rust/Axum, PostgreSQL/SQLx, frontend Next.js, JWT, roles `admin`, `empresa`, `ong` y `ceo`, RBAC, ownership y lifecycle de donaciones. Hay pruebas Rust de rutas, autorización y estados, además de Jest para login, registro, permisos, envíos y regresiones. Los conteos **más recientes documentados** son **74 tests Rust PASS** y **44 tests Jest PASS** tras FERXARP-UI-01 (ACT-03 registró 39); los **73/35** de ACT-01/02 son registros históricos anteriores a las regresiones. Esta auditoría no volvió a ejecutar las suites.

| Cobertura | Resultado documentado | Alcance exacto |
|---|---:|---|
| Rust, módulo escolar | **85.91% líneas** | Archivos definidos en ACT-01; supera el umbral escolar de 80% |
| Rust, reporte completo ACT-02 | **74.48% líneas** | Medición local histórica sin el recorte del módulo; no alcanza 80% |
| Jest final FERXARP-UI-01 | **98.04% líneas/sentencias**, **91.88% ramas**, **86.2% funciones** | Alcance instrumentado/configurado de Jest, no todo el frontend |
| SonarQube total ACT-03 | **77.8% frontend**, **77.9% backend** | Proyectos completos analizados por Sonar; no sustituye la cobertura del módulo escolar |

El [resumen Jest de ACT-01](../evidencias/testing/jest-coverage-summary.json) conserva el estado histórico; ACT-03 conserva su medición histórica en [calidad](../evidencias/quality/README.md). La validación final FERXARP-UI-01 confirmó 44 tests y los porcentajes Jest de la tabla; sus [evidencias UI](../evidencias/ui/) incluyen capturas y [resultados del navegador](../evidencias/ui/ferxarp-ui01-browser-results.json). La diferencia entre porcentajes refleja herramientas, momentos y alcances distintos.

**Frontend final — Red Verde Solidaria:** lint, typecheck, tests/cobertura, build Next y build/arranque Docker PASS según la validación FERXARP-UI-01. Responsive comprobada en **1440, 1280, 768 y 390 px**. El smoke funcional de navegador validó login, registro Empresa/ONG, logout, creación, matching, solicitud, aprobación, salida, entrega/rechazo, verificación/revocación ONG, mapa, CEO y navegación móvil. “Tu ubicación” solo aparece al recibir coordenadas válidas de `navigator.geolocation`; sin permiso no hay marcador personal ficticio y Veracruz se identifica como almacén demo/referencia. El caso concedido se probó con coordenadas simuladas en el navegador, no con GPS físico. Esta evidencia no equivale a una suite E2E completa en CI ni a nuevos scans Sonar/ZAP.

## 2. CI/CD

El [workflow](../../.github/workflows/ci-cd.yml) contiene jobs `backend-quality`, `frontend-quality`, `docker-build` y `deploy-test-environment`. Configura tests, cobertura, checks, builds Rust/Next y construcción Docker. El deploy-test corre solo en push a `main`, crea PostgreSQL, Chroma, backend y frontend, aplica migraciones, provisiona Admin de CI y espera readiness. [`smoke-test.sh`](../../scripts/smoke-test.sh) verifica **5 respuestas HTTP 200**: Chroma, `/health`, portada, login Admin y ruta Admin con JWT. [`deploy-test.sh`](../../scripts/deploy-test.sh) ejecuta `docker compose down -v --remove-orphans` al salir, incluso ante fallo.

La [validación local de ACT-02](../evidencias/cicd/validacion-local-2026-09-26.md) registra builds, cuatro servicios saludables, smoke **5/5 PASS** y limpieza. La [evidencia remota](../evidencias/cicd/github-actions-remote.md) registra además el [PR #1, run `36277313303`](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36277313303), con tests/builds PASS y deploy omitido según política, y el [push a `main`, run `36278602691`](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36278602691), con tests, builds Docker, deploy-test, cinco smoke HTTP 200 y limpieza PASS. La consulta anterior se hizo a **otro repositorio** (`07malpicadaniel-del/FERXARP`); su resultado vacío no describía los runs de `Rocoyoyi1527/FERXARP-rodrigo`.

## 3. Seguridad y calidad

OWASP ZAP 2.17.0 realizó baseline pasivo y full scan activo anónimos sobre frontend/backend **locales** antes y después. Se contaron **tipos de alerta por host**, no instancias; los JSON versionados confirman:

| Fase | High | Medium | Low | Informational |
|---|---:|---:|---:|---:|
| Antes | 0 | 3 | 3 | 1 |
| Después | 0 | 1 | 0 | 1 |

El XSS almacenado del popup Leaflet fue **High en la prueba complementaria Chromium**, no un aviso detectado por ZAP: [`browser-xss-before.json`](../evidencias/security/browser-xss-before.json) registra ejecución y [`browser-xss-after.json`](../evidencias/security/browser-xss-after.json) ausencia de ejecución con el mismo payload. Se corrigió mediante nodos DOM y `textContent`, con regresión Jest. ZAP registró la corrección de CORS `*`, protección contra clickjacking, `X-Content-Type-Options` y exposición de `X-Powered-By`. Permanece **1 Medium por CSP ausente** en frontend, documentado como P1. Las pruebas SQLi dirigidas no observaron bypass en los inputs probados; no demuestran ausencia absoluta. El active scan ZAP **no fue autenticado**; hubo HTTP dirigido con JWT para rutas protegidas.

SonarQube Community Build **26.9.0.129388** y SonarScanner CLI **7.1.0.4889** analizaron TypeScript/TSX y Rust localmente. El Quality Gate predeterminado **Sonar way** terminó **PASS** en ambos proyectos, sin modificar sus condiciones. Las métricas finales conservadas en [`sonar-after-frontend.json`](../evidencias/quality/sonar-after-frontend.json) y [`sonar-after-backend.json`](../evidencias/quality/sonar-after-backend.json) son:

| Métrica | Frontend | Backend |
|---|---:|---:|
| Bugs / vulnerabilities / security hotspots | 0 / 0 / 0 | 0 / 0 / 0 |
| Code smells activos | 40 | 1 |
| Deuda técnica estimada | 250 min | 6 min |
| Cobertura total Sonar | 77.8% | 77.9% |
| Líneas duplicadas | 0.0% | 6.0% |
| Ratings de mantenibilidad / fiabilidad / seguridad | A / A / A | A / A / A |

El smell backend pendiente es `rust:S3776` crítico en `backend/src/api/seed.rs:73` (complejidad 22 frente a 15). La cobertura **de código nuevo**, usada por el gate final, fue 93.1% frontend y 100.0% backend; un gate PASS no significa que la cobertura total sea ≥80%. Sonar no detectó el XSS que sí mostró Chromium.

## 4. Cierre

El [informe de cierre](informe-cierre.md) contiene resumen ejecutivo, objetivos, alcance, cronología con fechas y commits de Git, comparación planificado/ejecutado, desviaciones demostradas, resultados técnicos, seguridad/calidad, lecciones, pendientes y conclusión. Distingue los rótulos “Semana 1–4” de una línea base fechada: el repositorio **no conserva calendario suficiente para cuantificar retrasos en días**, y el informe no inventa esa cifra.

## 5. Mejora continua

El [plan de mejora](plan-mejora-continua.md) establece **25 acciones** (8 Alta, 13 Media, 4 Baja) con problema, acción, KPI/meta, evidencia esperada, dependencias y horizonte relativo. Incluye baseline y metas sin confundir resultados futuros con mediciones actuales. Contiene **3 innovaciones** —optimización de rutas, pronóstico de demanda y ficha ESG trazable— con hipótesis, datos, KPI, riesgos y MVP. Su roadmap distingue corto, mediano y largo plazo. La acción CI-01 de acreditación remota está marcada como completada; las demás no se presentan como implementadas.

## 6. Evidencias

| Criterio | Requisito de nivel alto | Evidencia concreta | Archivo/ruta | Estado |
|---|---|---|---|---|
| 1. Implementación y seguridad | Módulo, JWT y roles operativos | Lifecycle, RBAC y ownership descritos y probados | [Testing ACT-01](../evidencias/testing/README.md); [cierre](informe-cierre.md) | CUMPLIDO |
| 1. Implementación y seguridad | Tests Rust/Jest y cobertura ≥80% en el módulo evaluado | Último conteo documentado 74 Rust/44 Jest PASS; módulo Rust 85.91% líneas; Jest configurado 98.04% líneas | [Testing ACT-01](../evidencias/testing/README.md); [calidad ACT-03](../evidencias/quality/README.md); [UI final](../evidencias/ui/); [cierre](informe-cierre.md) | CUMPLIDO |
| 2. CI/CD | Workflow con tests y builds automáticos | Jobs backend, frontend y Docker PASS en PR y `main` remotos | [Workflow](../../.github/workflows/ci-cd.yml); [runs remotos](../evidencias/cicd/github-actions-remote.md) | CUMPLIDO |
| 2. CI/CD | Deploy-test, readiness, smoke y limpieza | Cuatro servicios; 5/5 HTTP 200 y `down -v` en `main` remoto y validación local | [Run de `main`](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36278602691); [evidencia](../evidencias/cicd/github-actions-remote.md) | CUMPLIDO |
| 2. CI/CD | Ejecución remota real acreditada | PR `36277313303` PASS; `main` `36278602691` PASS; artefactos Rust/Jest verificados | [PR](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36277313303); [`main`](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36278602691) | CUMPLIDO |
| 3. Seguridad y calidad | Scans, hallazgos, correcciones y re-scan | ZAP 0/3/3/1 → 0/1/0/1; XSS Chromium corregido; CORS/headers corregidos | [Seguridad ACT-03](../evidencias/security/README.md); [JSON ZAP final](../evidencias/security/zap-after-frontend.json) | CUMPLIDO |
| 3. Seguridad y calidad | Riesgos residuales identificados | CSP Medium abierto; active scan autenticado completo pendiente | [Seguridad ACT-03](../evidencias/security/README.md); [plan](plan-mejora-continua.md) | PARCIAL |
| 3. Seguridad y calidad | SonarQube, Quality Gate y métricas | PASS frontend/backend; bugs, vulnerabilities, hotspots, smells, deuda, cobertura, duplicación y ratings | [Calidad ACT-03](../evidencias/quality/README.md); [JSON backend final](../evidencias/quality/sonar-after-backend.json) | CUMPLIDO |
| 4. Cierre | Comparación y lecciones sustentadas | Cronología Git, alcance, diferencias, incidentes y lecciones; sin retrasos inventados | [Informe de cierre](informe-cierre.md) | CUMPLIDO |
| 5. Mejora continua | Acciones medibles e innovación | 25 acciones, KPI/meta, prioridades, horizontes, 3 MVP innovadores y roadmap | [Plan de mejora](plan-mejora-continua.md) | CUMPLIDO |

| Criterio | Estado documental final | Base del estado |
|---|---|---|
| 1. Implementación y seguridad | CUMPLIDO | Módulo, roles, tests y umbral del módulo escolar |
| 2. CI/CD | CUMPLIDO | PR y `main` remotos PASS; deploy-test en `main` |
| 3. Seguridad y calidad | PARCIAL | Pruebas, correcciones y métricas acreditadas; CSP y scan autenticado completo pendientes |
| 4. Cierre | CUMPLIDO | Informe de cierre sustentado en Git y evidencias |
| 5. Mejora continua | CUMPLIDO | Acciones medibles, roadmap e innovaciones documentadas |

Los estados no son puntuaciones.

## 7. Pendientes no bloqueantes

| Clasificación | Pendiente | Motivo y acción de entrega |
|---|---|---|
| **RECOMENDABLE** | CSP frontend Medium y scan ZAP autenticado completo | Son riesgos conocidos, no se ocultan. La rúbrica ya tiene hallazgos y correcciones reales; no se consideran automáticamente bloqueantes para la entrega escolar. |
| **RECOMENDABLE** | Aclarar referencias del README a Supabase/Groq como capacidades actuales | El [plan](plan-mejora-continua.md) documenta que la distancia de matching usa origen fijo y que Groq real no se validó. Conviene describir el alcance demostrado antes de presentar la aplicación. La actualización UI ya aclaró en el README el alcance de geolocalización; no cambió la distancia del matching. |
| **NO BLOQUEANTE** | Suite E2E completa y repetible en CI (ya existe smoke funcional de navegador UI-01), complejidad del seed, métricas CEO validadas, hardening Groq e innovaciones | Son trabajo futuro documentado en el plan, no evidencia que se deba inventar o declarar completa ahora. |

## 8. Checklist de entrega

Casilla marcada significa **archivo o evidencia comprobada en este corte**, no ejecución nueva ni aprobación final. Las capturas de UI ya existen en [evidencias UI](../evidencias/ui/); las demás capturas sugeridas siguen siendo recomendaciones.

- [x] Informe final: [informe de cierre](informe-cierre.md) presente.
- [x] Plan de mejora: [plan](plan-mejora-continua.md) presente.
- [x] Evidencia testing: [resumen ACT-01](../evidencias/testing/README.md); conviene capturar consola Rust PASS, Jest PASS/cobertura y llvm-cov del módulo.
- [x] Evidencia CI/CD local: [resumen ACT-02](../evidencias/cicd/README.md); conviene capturar healthchecks, smoke 5/5 y salida `down -v`.
- [x] Evidencia de GitHub Actions **remoto en verde**: [PR](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36277313303) y [`main`](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36278602691) PASS, con deploy-test, smoke, limpieza y artefactos documentados.
- [x] Evidencia ZAP: [antes/después](../evidencias/security/README.md); conviene capturar tabla de alertas y prueba Chromium del XSS.
- [x] Evidencia Sonar: [métricas y gate](../evidencias/quality/README.md); conviene capturar Quality Gate PASS, smells, deuda, cobertura y duplicación de ambos proyectos.
- [x] Workflow: [archivo YAML](../../.github/workflows/ci-cd.yml) presente.
- [x] Smoke funcional de demo en navegador: [19 resultados PASS](../evidencias/ui/ferxarp-ui01-browser-results.json) y [capturas UI](../evidencias/ui/) de roles, lifecycle, mapa y responsive. No se afirma que exista una grabación manual completa.
- [x] README: [archivo](../../README.md) presente y revisado; geolocalización aclarada; conviene revisar las referencias Supabase/Groq al alcance comprobado.
- [x] Repositorio limpio en el pre-flight de esta actualización.
- [ ] Repositorio limpio al entregar: los cambios documentales de esta actualización quedan sin commit por instrucción expresa.
- [ ] Push final: no realizado ni autorizado en esta actividad.

**Verificación mínima:** archivos fuente y reportes JSON presentes y legibles; 14 secciones del plan, 3 innovaciones y enlaces relativos comprobados; sintaxis Bash de los scripts clave válida. En la actualización de evidencia remota se verificaron los dos runs y artefactos por GitHub CLI, y el log remoto de los cinco smoke checks y limpieza. En aquel corte no se ejecutaron tests, Docker, ZAP ni Sonar nuevamente. La actualización UI incorpora la validación posterior ya confirmada; mantiene las métricas históricas de seguridad/calidad y no repite esas suites ni scans.
