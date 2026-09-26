# Informe de cierre FERXARP

**Corte documental:** commit `9792bcd7c4842e812ea0313d2831a91924455942`, fechado el 26-09-2026 en Git. **Base de este informe:** historial y archivos versionados del repositorio. Este documento no asigna calificación ni estima horas de trabajo.

**Actualización documental posterior:** se incorporaron los runs remotos de PR y `main` en [evidencia GitHub Actions](../evidencias/cicd/github-actions-remote.md). Las métricas y demás hallazgos conservan su corte original.

## 1. Resumen ejecutivo

FERXARP dispone de backend Rust/Axum con PostgreSQL/SQLx, frontend Next.js, autenticación JWT, cuatro roles, control de propiedad, ciclo transaccional de donaciones, matching local/vectorial con ChromaDB y despliegue **de prueba** automatizado en GitHub Actions. ACT-01 dejó pruebas y cobertura; ACT-02 dejó el pipeline y una validación local del despliegue efímero; ACT-03 dejó análisis ZAP/SonarQube con resultados antes/después. Un XSS almacenado del popup del mapa fue demostrado en Chromium y corregido. Permanece un aviso Medium de CSP ausente; no se presenta como resuelto.

El repositorio demuestra builds, pruebas, smoke tests y scans **locales**. Además, el [PR #1](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36277313303) validó remotamente tests y builds, y el [push a `main`](https://github.com/Rocoyoyi1527/FERXARP-rodrigo/actions/runs/36278602691) validó tests, builds y deploy-test con smoke y limpieza. No hay evidencia de despliegue a producción. Las cifras de cobertura de este informe conservan el alcance de cada herramienta: módulo escolar Rust, conjunto Jest instrumentado o proyectos completos analizados por SonarQube.

Fuentes principales: [README del proyecto](../../README.md), [documentación de fases](../../documentacion.md), [ACT-01](../evidencias/testing/README.md), [ACT-02](../evidencias/cicd/README.md), [ACT-03 seguridad](../evidencias/security/README.md), [ACT-03 calidad](../evidencias/quality/README.md), [workflow](../../.github/workflows/ci-cd.yml), [migración inicial](../../backend/migrations/20260924000000_initial_schema.sql), [migración de donaciones](../../backend/migrations/20260925000000_donation_lifecycle.sql) y `git log` hasta `9792bcd`.

## 2. Objetivos planteados

`documentacion.md` organiza el alcance en cuatro rótulos: **Semana 1, Arquitectura de Datos y Motor Core**; **Semana 2, Frontend y Enlace de Identidad**; **Semana 3, Logística y Trazabilidad Geoespacial**; **Semana 4, Seguridad, Orquestación y Refactorización**. Describe Rust, modelo relacional, scoring, Next.js, identidad por rol, escaneo, Leaflet, RBAC y Docker Compose. El README vigente añade matching híbrido, ChromaDB, Groq opcional, tableros por rol y trazabilidad de donaciones.

Las actividades escolares posteriores se documentaron por separado: pruebas/cobertura (ACT-01), CI/CD con despliegue de prueba (ACT-02) y seguridad/calidad (ACT-03). No se encontró un cronograma original con fechas de inicio, fecha objetivo por fase, dependencias, horas previstas o historias de usuario fechadas. `documentacion.md` entró en Git con `f5e6c55` el 23-09-2026, después del commit inicial `bd3842b`; además se presenta como comparativa y cierre. Por ello sus rótulos son una **referencia de alcance y orden narrativo**, no una línea base aprobada de calendario.

## 3. Alcance ejecutado

- **Datos y backend:** esquema y metadata SQLx versionados; autenticación JWT; roles `admin`, `empresa`, `ong`, `ceo`; RBAC y ownership; provisión segura del primer Admin; donaciones y bitácora con transiciones transaccionales. Las pruebas incluyen casos de concurrencia y rollback.
- **Frontend:** login, registro, tableros por rol, inventario, envíos por etapa y mapa Leaflet. La UI se alineó con la separación entre aprobación y salida física.
- **Matching:** ChromaDB con índice derivado y reconstruible desde PostgreSQL, scoring local/híbrido y fallback léxico. Groq aparece como enriquecimiento opcional; las evidencias locales no ejercitan el servicio externo real.
- **Operación y evaluación:** workflow de tests, cobertura, builds y deploy-test efímero; healthchecks, smoke y limpieza `down -v`; ZAP pasivo/activo local, pruebas HTTP autenticadas dirigidas, Chromium para XSS y SonarQube para TypeScript y Rust.
- **Métricas CEO:** existen endpoint y componente de UI. El código calcula estimaciones ambientales y de beneficiarios mediante factores fijos; la evidencia de cobertura del módulo escolar excluye `api/metrics.rs`. No se demuestra validación independiente de esas estimaciones ni se afirma que el módulo esté ausente.

## 4. Cronología real

Fechas tomadas de los timestamps de autor mostrados por `git log --date=iso-strict`; se expresa solo el día registrado, sin convertirlo en duración de tarea.

| Fecha | Fase | Commit | Cambio principal | Resultado observable |
|---|---|---|---|---|
| 2026-09-21 | Base | `bd3842b` | Monorepo, modelos Rust y configuración inicial | Estructura inicial versionada |
| 2026-09-23 | Funcionalidad | `cf7ed9e` | Actualización backend/frontend | Código de ambas capas versionado |
| 2026-09-23 | Matching/logística | `da4c2f7` | Escaneo, ChromaDB y tableros | Módulos iniciales incorporados |
| 2026-09-23 | Operación/UI | `e33adc3` | Compose, RBAC, logística y mapas | Orquestación y rutas presentes |
| 2026-09-23 | UI/documentación | `f5e6c55` | Interfaz y `documentacion.md` | Se registra la narrativa de cuatro semanas |
| 2026-09-23 | Roles | `6ad8847` | Tableros ONG/empresa y auditoría | Vistas por rol ampliadas |
| 2026-09-24 | IA | `b439a8c` | Matching con Groq y mapa | Integración opcional en código |
| 2026-09-24 | Compilación | `2d34c1d` | Elimina referencia huérfana a Groq | Compilación backend estabilizada según commit |
| 2026-09-24 | Datos | `f0c9a71` | Migración inicial y metadata SQLx | Base local reproducible versionada |
| 2026-09-24 | Seguridad | `0d2b7a4` | Autenticación, RBAC y ownership | Registro público limitado a empresa/ONG; pruebas de acceso |
| 2026-09-24 | Admin | `dde6263` | Bootstrap del primer Admin | CLI y tests de provisión incorporados |
| 2026-09-24 | Donaciones | `4c37389` | Lifecycle y migración transaccional | Aprobación separada de salida; concurrencia y bitácora probadas |
| 2026-09-24 | Frontend | `b281dd5` | UI alineada al lifecycle | Envíos y acciones por etapa actualizados |
| 2026-09-25 | Matching | `ea2673a` | Estabiliza Chroma y matching híbrido | API/colección consistentes, reindexación y tests de integración |
| 2026-09-25 | ACT-01 | `b20fd21` | Jest, cobertura y evidencia | 73 tests Rust, 35 Jest, módulo Rust 85.91% líneas |
| 2026-09-26 | ACT-02 | `0b8de8f` | Workflow y deploy-test | Builds/smoke locales PASS; runs remotos posteriores PR/main documentados en CI/CD |
| 2026-09-26 | ACT-03 | `9792bcd` | ZAP, correcciones, SonarQube | XSS corregido, gate final PASS, CSP pendiente |

## 5. Comparación planificado vs ejecutado

### Limitación de la comparación temporal

El repositorio no conserva una línea base temporal suficiente para cuantificar desviación en días. Los rótulos “Semana 1–4” de `documentacion.md` carecen de fechas de calendario y el archivo se versionó después de iniciada la implementación. **No se puede afirmar adelanto ni retraso de una cantidad de días.** La comparación siguiente mide alcance y secuencia observables, no puntualidad frente a un calendario inexistente.

| Elemento | Planificado/documentado | Ejecutado | Diferencia y estado | Motivo verificable | Impacto |
|---|---|---|---|---|---|
| Datos y motor core | Semana 1: modelo relacional y scoring Rust | Base inicial `bd3842b`; migración/SQLx reproducible `f0c9a71`; matching estabilizado `ea2673a` | **Ampliado** con migraciones e índice reconstruible; orden de commits distinto al rótulo semanal | Commits de baseline SQLx y estabilización Chroma | Builds y pruebas locales reproducibles |
| Frontend e identidad | Semana 2: Next.js, empresa y ONG | UI desde 23-09; JWT/RBAC endurecidos `0d2b7a4`; Admin/CEO y UI posterior | **Ampliado** a cuatro roles y provisión Admin | Diff de registro público y `dde6263` | Alta privilegiada separada del registro público |
| Logística y mapa | Semana 3: scanner y Leaflet | Versiones iniciales `da4c2f7`/`e33adc3`; lifecycle transaccional `4c37389`; UI `b281dd5` | **Modificado**: aprobación y salida son eventos distintos | Diff de `4c37389` y migración de lifecycle | Estados, bitácora y ownership coherentes |
| Seguridad y orquestación | Semana 4: RBAC y Compose | Compose/RBAC inicial 23-09; hardening 24-09; ZAP y correcciones 26-09 | **Ampliado** con pruebas dinámicas y headers | `0d2b7a4`, `9792bcd` y reportes ZAP | XSS/CORS corregidos; CSP aún abierto |
| Matching/IA | Scoring y vectorización; README menciona Groq | Chroma híbrido con fallback y Groq opcional | **Modificado/ampliado** para funcionar sin servicio externo | README y pruebas Chroma `ea2673a` | Matching local reproducible; Groq real fuera de la validación |
| Pruebas y cobertura | ACT-01 pide pruebas y umbral de módulo | 73 Rust/35 Jest en ACT-01; 74/39 tras ACT-03 | **Cumplido y ampliado** con regresiones de seguridad | Evidencias ACT-01/03 | Umbral de módulo y Jest acreditados sin mezclar alcances |
| CI/CD | ACT-02 pide tests, build y deploy de prueba | Workflow, despliegue efímero local y runs remotos PR/main con cinco smoke checks en `main` | **Cumplido local y remotamente** para el entorno de prueba | [Evidencia remota](../evidencias/cicd/github-actions-remote.md) y validación local ACT-02 | Tests/build en PR y `main`; deploy-test en `main` acreditado |
| Seguridad/calidad | ACT-03 pide ZAP y Sonar | Scans antes/después, XSS corregido, métricas y gate PASS | **Cumplido con pendiente** CSP Medium | Evidencias ACT-03 | Riesgo residual explícito |
| Métricas CEO | README presenta tablero y estimaciones | Endpoint/UI existentes con factores fijos | **Implementado; validación específica no acreditada** | `api/metrics.rs` y exclusión de ACT-01 | No confundir existencia con precisión demostrada |
| Producción | El plan de cuatro semanas menciona Compose, no un deploy productivo fechado | Solo deploy-test local y remoto; sin producción demostrada | **Fuera del alcance verificado** | Workflow condicionado a `main` despliega entorno efímero | No se atribuye operación productiva |

## 6. Desviaciones e incidentes

| Problema demostrado | Impacto observado o riesgo | Acción correctiva registrada | Resultado | Lección concreta |
|---|---|---|---|---|
| Referencia backend huérfana a Groq (`2d34c1d`) | Compilación inestable según el commit | Se retiró la referencia | Commit declara compilación estabilizada | Un build debe acompañar cambios de módulos/imports |
| El esquema/metadata SQLx no estaban versionados en el commit inicial | Checkout inicial dependía de estado de base no documentado; no se cuantifica fallo | Migración inicial y `.sqlx` en `f0c9a71` | ACT-01/02 validaron compilación y `prepare --check` | Esquema y metadata deben viajar con el código |
| Registro aceptaba el enum general `Role` antes de `0d2b7a4` | Riesgo de solicitar roles privilegiados por el endpoint público | `PublicRole` solo `empresa`/`ong` y tests | Admin se provisiona por CLI separado | RBAC debe imponer límites en backend |
| Aprobación de solicitud cambiaba la donación a `en_transito` antes de `4c37389` | Aprobación administrativa equivalía a salida física | Transiciones separadas, locks y pruebas de concurrencia | Salida exige solicitud aprobada | Eventos de negocio diferentes requieren estados diferentes |
| Cliente Chroma mezclaba rutas API/colección en el diff de `ea2673a` | Riesgo de indexar/consultar una colección incompatible | API v2 consistente, métrica coseno validada y reindexación | Tests Chroma y fallback local | Un índice derivado debe poder reconstruirse desde PostgreSQL |
| Primer cálculo local de cobertura ACT-02 sin herramientas LLVM configuradas | Primer intento falló; no se atribuye al código | Se configuraron herramientas LLVM del host | Ejecución final PASS documentada | El entorno de medición forma parte de la reproducibilidad |
| Popup Leaflet interpolaba nombre ONG como HTML | XSS almacenado ejecutado en Chromium | Nodos DOM y `textContent`, test Jest y re-scan | Chromium final sin ejecución; ZAP no lo detectó | Escáner automático y prueba de navegador se complementan |
| Quality Gate frontend falló de forma intermedia | `new_coverage=64.3% < 80%` | Tests `GardenGraph` y nuevo análisis | 93.1% de cobertura nueva; gate PASS | Medir cambios nuevos además del baseline |
| CSP ausente en el frontend | ZAP conserva aviso Medium | No se introdujo una CSP incompleta que rompiera Next/mapa | **Pendiente P1** | Una política debe verificarse con nonces y orígenes reales |

Estas filas no implican duración, costo ni culpa atribuible a una persona. Cuando el repositorio solo prueba un cambio de código, se describe el riesgo y el cambio, no un incidente operativo no registrado.

## 7. Resultados técnicos

| Medición/validación | Valor documentado | Alcance preciso |
|---|---|---|
| Tests Rust al cierre | **74 PASS** | Suite final ACT-03; ACT-01/02 registraron 73 antes de la nueva prueba CORS |
| Tests Jest al cierre | **39 PASS** | Suite final ACT-03; ACT-01/02 registraron 35 antes de las regresiones |
| Cobertura Rust módulo escolar | **85.91% líneas** | Archivos definidos por [ACT-01](../evidencias/testing/README.md); no es todo el backend |
| Cobertura Rust completa ACT-02 | **74.48% líneas** | Reporte completo de esa ejecución local; valor histórico, no sustituye medición Sonar ACT-03 |
| Cobertura Jest ACT-03 | **98.03% líneas; 90.27% ramas** | Conjunto instrumentado por Jest, no todo `frontend/src` |
| Build Rust/Next/Docker | **PASS local y remoto** | Validaciones ACT-01 a ACT-03; jobs de PR y `main` en [GitHub Actions](../evidencias/cicd/github-actions-remote.md) |
| Deploy-test ACT-02 | **PASS local y remoto** | PostgreSQL, Chroma, backend y frontend saludables; cinco respuestas smoke HTTP 200; `down -v` en `main` |

El [workflow](../../.github/workflows/ci-cd.yml) define jobs de backend, frontend, build Docker y despliegue efímero en push a `main`. La ejecución local del script consta en [validación ACT-02](../evidencias/cicd/validacion-local-2026-09-26.md); los runs remotos de PR y `main` constan por separado en [evidencia GitHub Actions](../evidencias/cicd/github-actions-remote.md).

## 8. Seguridad y calidad

ZAP baseline y active exploraron frontend/backend locales antes y después. Conteo de **tipos de alerta por host**: High **0→0**, Medium **3→1**, Low **3→0**, Informational **1→1**. El Medium restante es CSP. Además, la prueba complementaria de Chromium demostró un **XSS almacenado High**, corregido y reprobado con el mismo payload. Las solicitudes SQLi controladas sobre login, registro, donaciones, filtros, IDs, scanner y tracking no mostraron bypass en los inputs ensayados; no prueban ausencia absoluta de SQLi. Los scans ZAP fueron anónimos; las rutas autenticadas se probaron por HTTP dirigido, no mediante active scan autenticado completo. Ver [matriz y JSON ZAP](../evidencias/security/README.md).

SonarQube Community Build 26.9.0.129388 analizó TypeScript y Rust. El Quality Gate predeterminado **Sonar way** terminó **PASS** en ambos proyectos sin rebajar condiciones. Los valores finales fueron:

| Métrica Sonar | Frontend antes → después | Backend antes → después |
|---|---:|---:|
| Bugs / vulnerabilities / hotspots | 0/0/0 → 0/0/0 | 0/0/0 → 0/0/0 |
| Code smells activos | **46→40** | 1→1 |
| Cobertura total del proyecto | **68.8%→77.8%** | 78.1%→77.9% |
| Duplicación | 0.0%→0.0% | 6.1%→6.0% |
| Deuda técnica estimada | **291→250 min** | 6→6 min |
| Ratings mantenimiento/fiabilidad/seguridad | A/A/A → A/A/A | A/A/A → A/A/A |

La cobertura **nueva** final del gate fue 93.1% frontend y 100% backend. Los porcentajes totales Sonar usan todos los archivos analizados; no se presentan como los porcentajes del módulo escolar o de Jest. Sonar no detectó el XSS demostrado con navegador. Queda abierto un code smell crítico de complejidad en `backend/src/api/seed.rs:73`. Las mediciones originales y finales se conservan en [ACT-03 calidad](../evidencias/quality/README.md).

## 9. Lecciones aprendidas

1. **Versionar el esquema junto al backend:** `f0c9a71` añadió la migración inicial y metadata SQLx después de los primeros módulos; ACT-01/02 pudieron comprobar `prepare --check` sobre una base reproducible.
2. **Limitar roles en el endpoint público:** el diff `0d2b7a4` sustituyó `Role` por `PublicRole` para registro; la provisión Admin quedó en un CLI probado (`dde6263`).
3. **Separar autorización y movimiento físico:** `4c37389` dejó la aprobación de empresa separada de la salida de scanner y añadió pruebas de carrera/rollback. El estado refleja el evento ocurrido.
4. **Reconstruir índices derivados:** `ea2673a` fijó rutas/colección Chroma y agregó `reindex_chroma`; PostgreSQL permanece como fuente de perfiles y necesidades.
5. **Medir cobertura por alcance:** ACT-01 documentó 85.91% del módulo escolar y un porcentaje menor del backend completo. ACT-03 mostró otra cifra para el proyecto completo Sonar; el número de tests por sí solo no expresa cobertura.
6. **Probar la salida real del navegador:** ZAP y Sonar informaron cero XSS, mientras Chromium ejecutó el payload del popup. La regresión DOM confirmó la corrección.
7. **Atender el Quality Gate sin alterar su umbral:** el FAIL intermedio de 64.3% de cobertura nueva se resolvió con pruebas, y la medición final fue 93.1%.
8. **Hacer efímera la validación de despliegue:** ACT-02 migró, aprovisionó Admin, esperó readiness, ejecutó cinco smoke checks y borró volúmenes; las ejecuciones local y remota de `main` acreditan la repetibilidad de la prueba y limitan datos residuales.
9. **Distinguir código presente de comportamiento validado:** Groq y métricas CEO existen, pero las pruebas de ACT-01/03 no acreditan servicio Groq real ni la precisión de las estimaciones de impacto.

## 10. Riesgos y pendientes

| Prioridad | Pendiente confirmado | Base y siguiente verificación posible |
|---|---|---|
| P1 | CSP frontend con nonces y política compatible con Next, API y mapa | ZAP-10038 sigue Medium en [ACT-03](../evidencias/security/README.md); comprobar renderizado y re-scan antes de cerrar |
| P2 | Complejidad de `api/seed.rs:73` | Sonar conserva `rust:S3776`, complejidad 22 frente a 15, deuda estimada 6 min |
| P2 | Active scan autenticado y E2E de navegador más amplio | ZAP fue anónimo; hubo HTTP dirigido y una prueba Chromium de XSS, sin suite E2E completa |
| P2 | Validación de integración Groq y de estimaciones CEO | Groq fue opcional/no invocado en pruebas; métricas CEO usan factores fijos y quedaron fuera del módulo de cobertura ACT-01 |

El despliegue de producción no se realizó ni se afirma como pendiente con fecha: el workflow documentado despliega exclusivamente una instancia de prueba. No se infiere un P0 adicional a partir de ausencia de pruebas.

## 11. Conclusión

El cierre documental acredita el alcance funcional, la calidad y la seguridad **en los ambientes y rutas efectivamente probados**. Git permite reconstruir la secuencia de implementación y las evidencias permiten separar resultados iniciales, correcciones y resultados finales. La comparación temporal queda limitada porque no existe un calendario original fechado; este informe conserva esa incertidumbre en lugar de inventar retrasos. Los runs remotos acreditan el pipeline de prueba. La CSP, las comprobaciones autenticadas/E2E más amplias y la validación de componentes opcionales quedan identificadas para trabajo posterior, sin presentarlas como completadas.
