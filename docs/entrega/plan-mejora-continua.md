# Plan de mejora continua FERXARP

**Punto de partida:** [Informe de cierre](informe-cierre.md), commit `fe3d118` de 26-09-2026. Este documento propone trabajo futuro; **ninguna acción se declara implementada**. Los horizontes son relativos y las metas son propuestas de aceptación, no resultados actuales ni fechas históricas.

## 1. Objetivo

Convertir hallazgos y vacíos de evidencia de ACT-01 a ACT-04 en acciones concretas, medibles y ordenadas. La prioridad atiende primero riesgos de seguridad, después confiabilidad y calidad, experiencia de uso e innovación. Cada acción exige evidencia antes de marcarse como cumplida. El plan no presupone presupuesto, duración de sprint ni proveedor de IA.

Fuentes: [cierre ACT-04](informe-cierre.md), [pruebas ACT-01](../evidencias/testing/README.md), [CI/CD ACT-02](../evidencias/cicd/README.md), [ZAP y HTTP ACT-03](../evidencias/security/README.md), [SonarQube ACT-03](../evidencias/quality/README.md), [README](../../README.md), [documentación de fases](../../documentacion.md) y Git hasta `fe3d118`. Para los límites de datos y matching se cotejó el código versionado en Git: [`donations.rs`](../../backend/src/api/donations.rs), [`metrics.rs`](../../backend/src/api/metrics.rs), [`matcher.rs`](../../backend/src/ai/matcher.rs) y [`groq.rs`](../../backend/src/ai/groq.rs). No se presupone que un sistema externo, un modelo o una fuente de datos estén disponibles.

## 2. Estado base

| Medición o hecho | Baseline verificable | Alcance/fuente |
|---|---|---|
| Vulnerabilidad XSS almacenado | Corregida y verificada de nuevo en Chromium | [ACT-03 seguridad](../evidencias/security/README.md); conservar regresión |
| ZAP por host y tipo | High **0**, Medium **1** (CSP), Low **0**, Informational **1** | Baseline/pasivo final; active conserva CSP |
| ZAP autenticado completo | Sin ejecución documentada | ZAP fue anónimo; hubo HTTP dirigido con JWT |
| Quality Gate Sonar way | **PASS** frontend y backend | Análisis local ACT-03; sin run remoto de Sonar en CI |
| Code smells y deuda | Frontend **40**, **250 min**; backend **1**, **6 min** | Sonar final; `rust:S3776` crítico en seed |
| Cobertura Sonar total | Frontend **77.8%**, backend **77.9%** | Proyectos completos, no umbral escolar |
| Cobertura módulo escolar Rust | **85.91% líneas** | Alcance fijado por ACT-01 |
| Cobertura Jest final | **98.03% líneas**, **90.27% ramas** | Archivos instrumentados en ACT-03, no todo frontend |
| Deploy-test | **5/5** smoke local y limpieza `down -v` | ACT-02; sin run GitHub versionado |
| E2E navegador completo | Sin suite de lifecycle documentada | Existe una prueba Chromium dirigida al XSS |
| Métricas CEO | `quantity` sin unidad explícita; CO2e `×2.5`; beneficiarios `÷5` | Código versionado; no se acredita validez de factores |
| Distancia/urgencia matching | Origen fijo `19.1738, -96.1342`; `urgency_level: 4` al formar candidatos | `donations.rs`; no se afirma distancia o urgencia real |
| Embeddings | Vectores locales deterministas con alcance semántico limitado | README, sección ChromaDB |
| Groq | Modelo literal `deepseek-r1-distill-llama-70b`; integración externa no validada en ACT-01/03 | Código/README; **no hay prueba aquí de que el modelo esté obsoleto** |

La medición de 85.91% corresponde solo al módulo escolar Rust; 98.03% a los archivos instrumentados por Jest; 77.8%/77.9% a los proyectos completos de Sonar. No se promedian ni se intercambian estos porcentajes.

## 3. Principios de mejora

1. Conservar el matching local determinista y el fallback cuando falten Chroma o Groq. Una respuesta generativa puede explicar, pero no decidir en secreto el orden de candidatos.
2. Medir antes de prometer una mejora: cuando no existe baseline, la primera entrega es instrumentarlo y publicar la medición.
3. Usar datos de prueba y ambientes efímeros para scans y E2E; no dirigir herramientas activas a servicios externos.
4. Cerrar cada acción con artefactos verificables (tests, JSON, run de CI, decisión metodológica o acta de validación), sin reducir umbrales para obtener PASS.
5. Tratar las unidades, el origen geográfico y la urgencia como datos de negocio explícitos; mostrar “no disponible” cuando no hay medición válida.

## 4. Acciones priorizadas

Las fichas de las secciones 5–10 contienen problema, acción, KPI, meta, evidencia y dependencias. Aquí se ve el orden de ejecución propuesto.

| Prioridad | Acciones | Total |
|---|---|---:|
| **Alta** | SEC-01, SEC-02, QUAL-02, TEST-01, TEST-02, TEST-03, CI-01, CI-02 | **8** |
| **Media** | SEC-03, QUAL-01, QUAL-03, CI-03, PROD-01, DATA-01, DATA-02, DATA-03, GEO-01, URG-01, AI-01, AI-02, OPS-01 | **13** |
| **Baja** | AI-03, INNO-01, INNO-02, INNO-03 | **4** |

**Alta** atiende exposición o evidencia de confiabilidad. **Media** corrige deuda y datos que condicionan decisiones. **Baja** reserva experimentos para después de medir y estabilizar sus insumos. Dentro de cada prioridad, las dependencias de las fichas determinan el orden real.

## 5. Seguridad

**SEC-01 — Seguridad · Alta · Corto plazo.** Problema: ZAP conserva una alerta Medium por CSP ausente en el frontend. Acción: diseñar CSP con nonce para Next y orígenes mínimos compatibles con API, Leaflet y teselas; verificar páginas, mapa y autenticación con navegador de prueba.
- **KPI y meta:** tipos de alerta ZAP Medium por CSP **1 → 0**, sin fallos en smoke ni en las rutas `/`, `/login`, `/dashboard`, `/shipments`, `/map`.
- **Evidencia:** política documentada, test HTTP de encabezado, resultado E2E y JSON ZAP posterior.
- **Dependencias:** frontend de prueba y suite TEST-01; definir orígenes permitidos antes de activar la política.

**SEC-02 — Seguridad · Alta · Corto plazo.** Problema: los scans ZAP de ACT-03 fueron anónimos; los endpoints protegidos tuvieron solo HTTP dirigido. Acción: inventariar rutas protegidas por rol y ejecutar baseline/active autenticado acotado en Compose efímero, con usuarios y datos descartables.
- **KPI y meta:** **100% de las rutas protegidas inventariadas** cubiertas por contexto autenticado o marcadas con exclusión justificada; cero blancos externos; cada hallazgo High/Medium clasificado y con decisión registrada.
- **Evidencia:** matriz ruta/rol, configuración de contexto sin tokens versionados, JSON ZAP, smoke posterior y `down -v`.
- **Dependencias:** cuentas de prueba, límites de mutación y SEC-01 para comparar avisos.

**SEC-03 — Seguridad · Media · Corto plazo.** Problema: no hay un inventario periódico de vulnerabilidades de dependencias en las evidencias citadas; esto **no** equivale a afirmar que existan vulnerabilidades conocidas. Acción: revisar lockfiles de Rust/Node y proponer actualizaciones mediante una herramienta mantenida o avisos de dependencias, con triage humano.
- **KPI y meta propuesta:** **0 hallazgos Critical/High sin triage** por ciclo; resolución o excepción documentada en un horizonte propuesto de 1 ciclo para Critical y 2 para High. El primer scan establece el baseline de hallazgos.
- **Evidencia:** reporte fechado, issue/PR por hallazgo y justificación de excepciones.
- **Dependencias:** inventario de dependencias y CI-01; no introducir actualizaciones automáticas a ciegas.

## 6. Calidad y pruebas

**QUAL-01 — Calidad de código · Media · Corto plazo.** Problema: Sonar conserva **1** issue crítico `rust:S3776` en `backend/src/api/seed.rs:73`, complejidad 22 frente a 15. Acción: extraer pasos del seed sin cambiar idempotencia, RBAC ni datos de demostración.
- **KPI y meta:** issues críticos `rust:S3776` de ese archivo **1 → 0**; tests existentes PASS y seed repetido sin duplicados.
- **Evidencia:** JSON Sonar posterior, diff acotado y pruebas de seed.
- **Dependencias:** entorno local PostgreSQL/Chroma y datos de prueba.

**QUAL-02 — Calidad de código · Alta · Mediano plazo.** Problema: el Quality Gate solo está acreditado localmente. Acción: exigir Sonar way sin rebajar condiciones en PR y `main`, con análisis de frontend y Rust y publicación del estado.
- **KPI y meta:** **PASS en cada PR y push a `main`** que se fusione; ningún FAIL omitido por configuración.
- **Evidencia:** enlaces de runs y estado de gate por commit, más snapshots de métricas.
- **Dependencias:** CI-01 y gestión segura de token de análisis.

**QUAL-03 — Calidad de código · Media · Mediano plazo.** Problema: Sonar final registra **250 min** de deuda frontend y **6 min** backend. Acción: seleccionar smells activos por impacto y corregirlos junto a cambios funcionales, sin refactor general.
- **KPI y meta propuesta:** deuda frontend **≤225 min** en dos ciclos de análisis; backend **≤6 min** y issue QUAL-01 cerrado. Registrar variación por ciclo.
- **Evidencia:** comparativo de JSON Sonar con lista de issues cerrados y nuevos.
- **Dependencias:** QUAL-01, QUAL-02; no reclasificar severidades para reducir la métrica.

**TEST-01 — Pruebas · Alta · Corto plazo.** Problema: hay tests de backend y una comprobación Chromium del XSS, pero no una suite E2E completa del lifecycle. Acción: automatizar con navegador y Compose local dos rutas: Admin verifica ONG → Empresa publica → ONG solicita → Empresa aprueba → salida → **entrega** o **rechazo**.
- **KPI y meta:** **2 escenarios terminales E2E PASS** en un entorno limpio por ejecución, con assertions de estado/bitácora/ownership.
- **Evidencia:** reporte E2E, logs de test y limpieza de volúmenes.
- **Dependencias:** fixtures por rol, CI-02 y datos sin servicios externos.

**TEST-02 — Pruebas · Alta · Corto plazo.** Problema: ZAP y Sonar no detectaron el XSS que Chromium sí mostró; las regresiones XSS/CORS existen, pero no hay regla para hallazgos futuros. Acción: acompañar cada corrección High/Medium con prueba automatizada del vector concreto y re-scan dirigido.
- **KPI y meta:** **100% de hallazgos High/Medium corregidos** con prueba de regresión y resultado posterior; mantener las pruebas existentes de XSS/CORS PASS.
- **Evidencia:** matriz hallazgo → test → scan antes/después.
- **Dependencias:** SEC-01/02 y clasificación reproducible del hallazgo.

**TEST-03 — Pruebas · Alta · Corto y mediano plazo.** Problema: la cobertura total Sonar (**77.8% frontend; 77.9% backend**) es menor que la del módulo escolar/Jest, por diferencia de alcance. Acción: conservar umbrales escolares y añadir tests útiles en archivos no cubiertos del proyecto completo.
- **KPI y meta propuesta:** módulo Rust **≥80% líneas** (base 85.91%); Jest **≥80% en sus cuatro métricas** (líneas 98.03%); cobertura total Sonar **≥80% por proyecto**, desde 77.8%/77.9%, sin excluir archivos para alterar el resultado.
- **Evidencia:** resúmenes llvm-cov/Jest, LCOV y medidas Sonar por commit.
- **Dependencias:** QUAL-02 y selección de pruebas por riesgo, no por número de líneas.

## 7. CI/CD y operación

**CI-01 — CI/CD · Alta · Corto plazo.** Problema: ACT-02 acredita ejecución local, pero el repositorio no conserva un run remoto de GitHub Actions. Acción: ejecutar el workflow tras publicación normal y archivar enlaces/artefactos de PR y `main`.
- **KPI y meta:** **1 PR** con test/build PASS y deploy omitido según diseño; **1 push a `main`** con los cuatro jobs PASS, incluidos deploy-test y limpieza. Baseline: **sin evidencia remota versionada**, no “cero runs existentes”.
- **Evidencia:** URL/ID de runs, artefactos de cobertura y logs de smoke.
- **Dependencias:** publicación autorizada por el equipo; no se propone hacerla como parte de ACT-05.

**CI-02 — CI/CD · Alta · Corto plazo.** Problema: el deploy-test pasó localmente y debe conservar su reproducibilidad. Acción: mantener checks de cuatro servicios, migración, provisión Admin y limpieza en cada despliegue de prueba.
- **KPI y meta:** smoke **5/5 HTTP 200** en cada `main` y `docker compose down -v` exitoso, incluso ante fallo; baseline local 5/5.
- **Evidencia:** job `deploy-test-environment`, estados de healthchecks y salida de limpieza.
- **Dependencias:** CI-01 y datos exclusivamente efímeros.

**CI-03 — CI/CD · Media · Mediano plazo.** Problema: Sonar y ZAP se ejecutaron localmente en ACT-03, no como gates del pipeline. Acción: integrar Sonar y ZAP baseline contra el deploy-test efímero, con scopes locales, token Sonar no versionado y hallazgos visibles.
- **KPI y meta:** cada PR/main reporta gate Sonar y baseline ZAP; **0 High** y **0 Medium sin decisión** para permitir merge; no silenciar reglas ni cambiar severidades.
- **Evidencia:** jobs, artefactos JSON, política de triage y run exitoso/fallido reproducible.
- **Dependencias:** SEC-01, SEC-02, QUAL-02 y CI-02; el active scan de mutaciones permanece aislado.

**OPS-01 — Observabilidad/operación · Media · Mediano plazo.** Problema: existen healthchecks/smoke, pero las evidencias no incluyen una serie de latencia y errores operativos. Acción: instrumentar salud y latencia de `/health`, login y matching en el entorno de prueba, sin datos personales en logs.
- **KPI y meta:** **100% de releases de prueba** con reporte de tasa de error y p95 de esos tres endpoints; **sin objetivo numérico de p95** hasta medir un baseline real.
- **Evidencia:** reporte de observabilidad por run y revisión de sanitización de logs.
- **Dependencias:** CI-02 y definición de muestras comparables.

## 8. Datos y métricas

**PROD-01 — Producto · Media · Mediano plazo.** Problema: el repositorio acredita flujos técnicos, no una validación documentada de comprensión de pantallas por Empresa y ONG. Acción: probar los dos flujos terminales con usuarios de prueba y registrar fricciones sin asumir que la UI actual sea defectuosa.
- **KPI y meta propuesta:** **2 sesiones por rol** (Empresa y ONG) con tareas completadas y observaciones clasificadas; las correcciones que se decidan tendrán criterio de aceptación propio.
- **Evidencia:** guion, resultados anonimizados y decisiones de producto.
- **Dependencias:** TEST-01 y consentimiento/datos de prueba.

**DATA-01 — Datos y métricas · Media · Mediano plazo.** Problema: la tabla guarda `quantity` entero sin unidad/peso, mientras el panel lo usa como volumen. Acción: modelar unidad y peso medido o conversión justificada, y distinguir registros históricos sin dato fiable.
- **KPI y meta:** **100% de nuevas donaciones** con unidad explícita validada; **0 registros sin peso verificable** usados para métricas expresadas en kg.
- **Evidencia:** migración, validaciones de API/UI, tests de unidades y reporte de registros excluidos.
- **Dependencias:** reglas de negocio por tipo de excedente; no inventar pesos para datos antiguos.

**DATA-02 — Datos y métricas · Media · Mediano plazo.** Problema: `api/metrics.rs` multiplica cantidad entregada por **2.5** para CO2e sin metodología acreditada en las evidencias. Acción: definir alcance, unidad y factores con fuentes/versiones y revisión de dominio; solo entonces reemplazar el cálculo.
- **KPI y meta:** **100% de factores usados** con fuente, vigencia y unidad documentadas; **0 resultados CO2e** derivados de cantidad sin unidad válida.
- **Evidencia:** metodología aprobada, tabla de factores versionada y tests de cálculo.
- **Dependencias:** DATA-01; ninguna fórmula futura se presupone en este plan.

**DATA-03 — Datos y métricas · Media · Mediano plazo.** Problema: beneficiarios se estiman como cantidad entregada **÷5** sin validación acreditada. Acción: definir captura de beneficiarios o una metodología de estimación explícita y distinguir dato observado de estimado.
- **KPI y meta:** **0 cifras de beneficiarios** provenientes de una constante sin fuente; **100% de cifras publicadas** etiquetadas como observadas o estimadas y trazables al método.
- **Evidencia:** metodología, interfaz revisada, datos de prueba y tests.
- **Dependencias:** DATA-01 y decisión de privacidad de ONG.

**GEO-01 — Datos y métricas · Media · Mediano plazo.** Problema: el matching usa un origen fijo `19.1738, -96.1342` para donaciones; la ubicación real de recogida no está modelada en ese cálculo. Acción: capturar coordenadas verificadas del punto de recogida y calcular Haversine desde ellas; usar “distancia no disponible” si faltan.
- **KPI y meta:** **100% de valores `distance_km` publicados** derivados de coordenadas reales validadas de origen y destino; **0 distancias** atribuidas al origen fijo.
- **Evidencia:** migración/contrato, tests de coordenadas ausentes/inválidas y comparación de rutas antes/después.
- **Dependencias:** consentimiento para ubicación y DATA-01 si se optimizan cargas; no implica GPS en tiempo real.

**URG-01 — Datos y métricas · Media · Mediano plazo.** Problema: `donations.rs` asigna `urgency_level: 4` a cada candidata ONG aunque el scoring acepta valores 1–5. Acción: definir quién declara urgencia, vigencia y validación; mostrar “sin dato” y usar una política neutra documentada cuando falte.
- **KPI y meta:** **0 candidatas** con urgencia presentada como declarada si procede de un valor hardcodeado; **100% de urgencias utilizadas** con fuente y fecha o marcadas como desconocidas.
- **Evidencia:** contrato de datos, tests de ranking con/sin urgencia y auditoría de registros.
- **Dependencias:** validación con ONG y criterio de actualización; no inferir urgencia mediante IA sin revisión.

## 9. Inteligencia artificial

**AI-01 — Inteligencia artificial · Media · Mediano plazo.** Problema: el identificador de modelo Groq está fijado en código y la integración real no fue validada en ACT-01/03. **El repositorio no demuestra que ese modelo esté obsoleto**; su vigencia debe verificarse al ejecutar la acción. Acción: seleccionar un modelo soportado y configurable, fijar timeout, validar respuesta estructurada y preservar fallback local.
- **KPI y meta:** configuración de modelo sin literal de producción en código; tests PASS para timeout, respuesta inválida y ausencia de credencial; **100% de esos fallos** terminan con matching local disponible.
- **Evidencia:** decisión de modelo con fecha/fuente, configuración, tests con servidor simulado y reporte de integración en entorno de prueba.
- **Dependencias:** secreto fuera del repositorio, política de costos y AI-02.

**AI-02 — Inteligencia artificial · Media · Mediano plazo.** Problema: el enriquecimiento generativo no cuenta con una regresión que pruebe explícitamente la independencia del ranking. Acción: limitar IA a explicación/etiquetas verificables después del ranking determinista.
- **KPI y meta:** para un dataset fijo, **100% de rankings idénticos** con IA activada, fallida o desactivada; ninguna transición de donación depende de Groq.
- **Evidencia:** tests de igualdad de ranking y trazas sin datos sensibles.
- **Dependencias:** baseline determinista actual, AI-01 y criterios de aceptación de explicaciones.

**AI-03 — Inteligencia artificial · Baja · Largo plazo.** Problema: el README reconoce que los vectores locales deterministas tienen alcance semántico limitado; no existe benchmark de relevancia publicado. Acción: comparar embeddings candidatos **sin comprometer proveedor** contra el matching léxico/vectorial actual en datos etiquetados.
- **KPI y meta experimental:** construir **≥30 consultas/pares etiquetados**, medir `NDCG@5` del baseline y del candidato; adoptar un candidato solo si mejora **≥5 puntos porcentuales propuestos** sin perder fallback ni protección de datos. El valor actual de `NDCG@5` es **no medido**.
- **Evidencia:** dataset anonimizado, protocolo, resultados reproducibles y decisión de adopción/rechazo.
- **Dependencias:** revisión de privacidad, costos y AI-02; no enviar perfiles reales a un proveedor por defecto.

## 10. Propuestas de innovación

Son **experimentos futuros**, no capacidades implementadas ni métricas actuales. Cada MVP depende primero de los datos y controles anteriores.

| ID / área / prioridad / horizonte | Problema e hipótesis | Datos requeridos | KPI y meta propuesta | Riesgos | MVP, evidencia y dependencias |
|---|---|---|---|---|---|
| **INNO-01 · Innovación · Baja · Largo** | La distancia hoy parte de un origen fijo; con puntos reales y ventanas de recogida, una agrupación de rutas podría reducir recorrido frente a asignación por cercanía simple. | GEO-01, ubicaciones consentidas, capacidad y ventanas de prueba. | Distancia total mediana por lote de prueba; **≥10% menos** que una regla base medida en el mismo dataset, sin aumentar entregas fuera de ventana. Baseline actual: **no medido**. | Datos geográficos incompletos, cargas incompatibles, exposición de ubicaciones. | Simulador offline con rutas de prueba, comparación reproducible y decisión; depende de GEO-01 y DATA-01. **No** promete tracking GPS. |
| **INNO-02 · Innovación · Baja · Largo** | La urgencia está fija; una serie real de necesidades podría anticipar demanda y orientar inventario sin sustituir la declaración de ONG. | URG-01, historial consentido de necesidades/entregas y fechas; volumen suficiente por evaluar. | Error absoluto medio de pronóstico contra baseline ingenuo; probar adopción solo con **≥10% menos error propuesto** y suficientes muestras. Baseline actual: **no medido**. | Series escasas, estacionalidad, sesgo entre ONG, exposición de necesidades. | Pronóstico offline para una categoría, revisión humana y reporte de error; depende de URG-01 y calidad de datos. |
| **INNO-03 · Innovación · Baja · Largo** | El panel CEO estima impacto con factores fijos; una ficha ESG trazable podría hacer auditables entregas y supuestos. | DATA-01/02/03, bitácora de entrega, factor versionado y permiso de publicación. | **100% de cifras del MVP** trazables a entrega, unidad y versión del factor; **0 cifras sin método**. Baseline: metodología verificable **no acreditada**. | Doble conteo, atribución de impacto, privacidad y falsa precisión. | Exportación de una ficha por donación entregada con marca “observado/estimado”; depende de metodología aprobada y revisión de ONG. |

## 11. Roadmap

| Horizonte relativo | Entregas propuestas | Criterio para avanzar |
|---|---|---|
| **Corto plazo** | SEC-01 CSP; SEC-02 scan autenticado acotado; TEST-01 E2E; TEST-02 regresiones; QUAL-01 seed; CI-01 evidencia remota; CI-02 smoke/limpieza; iniciar SEC-03 y TEST-03 | Sin High abiertos, CSP evaluada, dos flujos E2E PASS, seed issue cerrado, runs y smoke acreditados. |
| **Mediano plazo** | Completar TEST-03; QUAL-02 gate; QUAL-03 deuda; CI-03 seguridad/calidad en pipeline; DATA-01/02/03; GEO-01; URG-01; PROD-01; OPS-01; AI-01/02 | Datos y métodos auditables, Sonar total medido, pipeline con reportes y ranking independiente de Groq. |
| **Largo plazo** | AI-03 benchmark; INNO-01 rutas; INNO-02 demanda; INNO-03 ficha ESG | Experimentos comparados con baselines medidos; adoptar solo al superar métricas y revisión de riesgos. |

Los horizontes expresan **precedencia propuesta**, no semanas transcurridas ni fechas de entrega ya aprobadas. Si una dependencia falla, su acción posterior no se considera lista.

## 12. KPIs y seguimiento

| KPI | Baseline verificado | Meta propuesta | Frecuencia futura | Herramienta/evidencia |
|---|---:|---:|---|---|
| Quality Gate frontend/backend | PASS / PASS local | PASS en cada PR y `main` | Cada cambio | SonarQube y CI |
| ZAP High | 0 tipos | 0 | Cada release de prueba | ZAP JSON |
| ZAP Medium | 1 tipo, CSP | 0 tras SEC-01 | Tras hardening y cada release | ZAP JSON |
| Coverage módulo Rust, líneas | 85.91% | ≥80% | Cada CI | llvm-cov módulo escolar |
| Coverage Jest, líneas | 98.03% | ≥80%; conservar también umbrales de ramas, funciones e instrucciones | Cada CI | Jest instrumentado |
| Coverage Sonar total | Frontend 77.8%; backend 77.9% | ≥80% por proyecto | Cada PR | SonarQube/LCOV |
| Code smells activos | Frontend 40; backend 1 | Seed crítico 0; reducir frontend por priorización | Cada análisis | SonarQube |
| Deuda técnica | Frontend 250 min; backend 6 min | Frontend ≤225 min; backend ≤6 min | Cada ciclo de análisis | SonarQube |
| Smoke deploy-test | 5/5 local | 5/5 y limpieza PASS | Cada `main` | GitHub Actions + Compose |
| E2E lifecycle navegador | Sin suite completa | 2 terminales PASS | Cada `main` | Runner E2E |
| Run remoto versionado | Sin evidencia | 1 PR y 1 `main` acreditados | Primera publicación y cambios posteriores | GitHub Actions |
| Origen fijo en distancias | `19.1738, -96.1342` | 0 distancias publicadas desde origen fijo | Cada cambio de matching | Tests/API y auditoría de datos |
| Urgencia hardcodeada | `urgency_level: 4` | 0 urgencias declaradas ficticiamente | Cada cambio de matching | Tests y fuente de dato ONG |
| Relevancia de embeddings (`NDCG@5`) | No medida | Medir baseline; ≥5 pp de mejora antes de adopción | Por experimento | Dataset etiquetado y benchmark |

En cada ciclo se guarda baseline, resultado, decisión y enlace a evidencia. Una meta no alcanzada queda abierta con causa registrada; no se rebaja la severidad ni el umbral para cambiar su estado. Las metas de 225 min, 80% Sonar y mejoras experimentales son **objetivos futuros**, no resultados observados.

## 13. Riesgos

- **CSP:** una política incompleta puede romper hidratación de Next, mapa o llamadas API. Validar en navegador y usar reportes ZAP antes de exigirla.
- **Scans autenticados:** las mutaciones deben limitarse al Compose descartable, sin fuerza bruta ni servicios externos.
- **Datos de impacto:** convertir cantidad a kg o personas sin unidad/fuente produciría cifras engañosas; excluir datos no verificables hasta aprobar metodología.
- **Geolocalización y urgencia:** ubicar a una ONG o inferir necesidad puede revelar información sensible o sesgar prioridades; exigir consentimiento, control por rol y opción “sin dato”.
- **IA y predicción:** modelos externos pueden cambiar o fallar, aumentar costo o introducir sesgo. Mantener fallback y ranking determinista; benchmark y revisión humana antes de adoptar innovaciones.
- **CI remoto:** no inferir éxito de GitHub Actions a partir del smoke local; la evidencia remota sigue siendo un entregable distinto.

## 14. Conclusión

El plan traduce el cierre en **25 acciones** con indicador, meta, evidencia, dependencia y horizonte: 8 Alta, 13 Media y 4 Baja. Primero se cierra la exposición CSP y se amplía la verificación de seguridad/confiabilidad; después se corrigen calidad y semántica de datos; por último se evalúan innovaciones frente a baselines medidos. Ninguna propuesta se presenta como implementada ni se asigna calificación.
