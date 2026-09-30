# FERXARP-DEMO-DATA

Auditoría y ampliación local, 29 de septiembre de 2026. Rama `feat/demo-data-veracruz`, punto de partida `73d0ff22e4d6539457ea3643fae4aca0e9130fed`, working tree inicialmente limpio. No commit ni push. La presentación web no se modifica.

## Scraper e historia verificable

**No se encontró un scraper real en el árbol actual ni en los 29 commits alcanzables desde las referencias locales.** No hay evidencia de un archivo scraper/crawler eliminado o reemplazado. Esto no prueba la inexistencia de scripts externos, ramas no disponibles o trabajo nunca versionado.

Se inspeccionaron `backend/src/api/seed.rs`, `backend/src/ai/index.rs`, todos los binarios, `scripts/`, dependencias y cambios históricos. Búsquedas actuales e históricas: scraper/scrap/crawler/crawl, reqwest, Html::parse, Selector, Veracruz, organizaciones/organizations/ONG y seed. Los `Selector`/`querySelector` encontrados corresponden al DOM y a pruebas de navegador; no a descubrimiento de ONG.

| Commit | Archivo / cambio | Hecho comprobado |
|---|---|---|
| `b439a8c` · 24 sep | `backend/src/api/seed.rs` | Introduce el endpoint `/api/seed/veracruz`: listas literales con 3 empresas, 5 ONG y 6 donaciones. Inserta PostgreSQL, prepara índice vectorial y evalúa compatibilidad mediante Groq. No consulta directorios ni extrae HTML. |
| `2d34c1d` · 24 sep | elimina `backend/src/api/groq.rs` | Elimina una referencia/módulo huérfano de Groq; no un scraper. |
| `f0c9a71` · 24 sep | baseline PostgreSQL/SQLx | Hace reproducible el esquema y ajusta el seed; no incorpora scraping. |
| `0d2b7a4` · 24 sep | `seed.rs` | Exige Admin y contraseña de entorno; deja de verificar automáticamente las ONG del antiguo listado. |
| `ea2673a` · 25 sep | `ai/index.rs`, `bin/reindex_chroma.rs`, `seed.rs` | Centraliza upserts por UUID, añade reindexación desde PostgreSQL y permite omitir Groq con `include_ai=false`. |

`reqwest` atiende APIs de ChromaDB y Groq. El flujo actual recibe perfiles por registro o seed y reconstruye un índice derivado; no descubre organizaciones en Internet. No existe una causa documentada de retirada de un scraper: no se encontró tal retirada.

El listado antiguo usaba nombres de empresas/organizaciones que parecían reales sin fuentes de descubrimiento. Los nombres/coordenadas de empresas ni siquiera se persistían, porque no existe un perfil empresarial con esos campos. Las seis donaciones partían en acopio. Su idempotencia dependía de correo/título, sin transacción global; algunos errores SQL se descartaban. La ampliación reemplaza ese listado de código por fixtures inequívocamente ficticios, sin borrar registros ya existentes.

## Conteos locales

Fuente: PostgreSQL del Compose local, base `ferxarp`. Activas = `en_acopio`, `reservado`, `en_transito`.

| Entidad | Antes | Después |
|---|---:|---:|
| ONG | 1 | 11 |
| Usuarios Empresa | 1 | 7 |
| Donaciones | 1 | 21 |
| ONG con ambas coordenadas | 1 | 11 |
| Donaciones activas | 0 | 16 |

Se conservan íntegros los registros previos de users, ngos, donations, donation_requests y delivery_logs, comprobando sus valores antes/después. La donación entregada anterior explica por qué el total final de entregadas es 3, aunque el fixture agrega 2.

## Dataset y seguridad de la carga

10 ONG + 6 empresas + 20 donaciones; catálogo en `backend/src/api/seed/data.rs`. Coordenadas aproximadas en Veracruz, Boca del Río, Medellín y puerto/centro. No son domicilios validados.

- Nombres ONG y títulos incluyen **Demo**; descripciones y logs declaran **DEMO FICTICIA**. Correos exclusivamente `@demo.ferxarp.invalid`; ninguna entidad afirma ser cliente o contraparte real.
- Nueve ONG demo comienzan con verificación **simulada para el ensayo**; la décima queda pendiente para mostrar al Admin. No es verificación de una organización real. Una revocación posterior no se revierte al repetir el seed.
- Empresas representadas como usuarios `empresa` con correos demo descriptivos; no se inventa un campo de razón social inexistente.
- Categorías: alimentos, leche, limpieza, material escolar, laptops, monitores, escritorios, sillas, herramientas, tarimas y uniformes. Cantidades de ejemplo; no certifican peso ni impacto.
- Estados del fixture: 8 en acopio; 4 reservadas (2 pendientes y 2 aprobadas); 4 en tránsito; 2 entregadas; 2 rechazadas. Se crean 12 solicitudes y 12 logs físicos simulados coherentes. Aprobación mantiene reservado; salida inicia tránsito. Entrega/rechazo tienen completed_at y rechazo tiene motivo.
- UUID estables en un namespace de fixtures, transacción global y bloqueo advisory transaccional. No adopta usuarios ajenos por correo ni cambia roles/contraseñas. Colisiones abortan la transacción.
- Repetir o ejecutar dos cargas concurrentes conserva registros, progresos y logs. No es una operación de reset. No se borran los fixtures para reiniciar una demostración.
- Se reutiliza `DonationState::after_physical_action` para validar la secuencia física. No se cambia el lifecycle ni sus permisos.
- Groq desactivado por defecto, opcional con `include_ai=true`; no se invocó en esta carga. No hay scraping en vivo.

## Reproducir y ensayar

1. Levantar PostgreSQL, ChromaDB y la aplicación con el Compose existente y aplicar sus migraciones habituales.
2. Definir `FERXARP_SEED_PASSWORD` en `backend/.env` antes de iniciar/recrear el backend. En esta ejecución se generó una contraseña aleatoria local en ese archivo ignorado por Git, con permisos 0600. No se publica su valor.
3. Iniciar sesión como Admin y ejecutar `POST /api/seed/veracruz?include_ai=false` con su JWT, o usar «Cargar datos demo» en el panel Admin. El endpoint conserva RBAC; una configuración ausente no modifica datos.
4. Reindexar desde `backend/`: `SQLX_OFFLINE=true cargo run --locked --bin reindex_chroma`.

Cuentas de ensayo recomendadas, con la contraseña local del seed:

- Empresa: `alimentos-veracruz@demo.ferxarp.invalid`.
- ONG: `banco-veracruz@demo.ferxarp.invalid`.
- ONG pendiente para verificación: `comunidad-centro@demo.ferxarp.invalid`.

La Empresa principal tiene 8 lotes y 6 solicitudes/envíos visibles. La ONG principal tiene 6 solicitudes/envíos propios y el catálogo contiene 8 lotes disponibles. «Demo · Alimentos en tránsito» permite demostrar recepción; «Demo · Leche para revisión» es otro lote para rechazo. No entregar y rechazar el mismo lote. Las sesiones Empresa/ONG deben abrirse en contextos de navegador separados.

El seed no modifica datos anteriores ni resets de ensayo. Para otros entornos, si ya existen cuentas con los correos/UUID reservados que no corresponden al fixture, la carga falla y requiere revisión, no reasignación automática.

## Mapa y matching

- 11 ONG con coordenadas + 1 almacén demo. El encuadre inicial y «Ver Veracruz» se ajustan a los puntos registrados para incluir Medellín; no se cambian coordenadas para hacerlas caber.
- El almacén conserva su identificación demo; conexiones de referencia, no rutas ni tracking. Con permiso denegado no aparece marcador de usuario. La ruta con permiso usa únicamente coordenadas devueltas por el navegador, cubierta por la prueba existente.
- Reindexación existente: 11 procesadas, 11 indexadas, 0 fallidas; consulta por IDs acredita una sola entrada por ONG.
- Matching local del lote de leche: modo `hybrid`, 11 candidatos distintos, todos presentes en PostgreSQL. No es una medición de precisión. La aprobación sigue siendo humana y las solicitudes requieren ONG verificada.
- Prueba reforzada con el propio dataset: más de un candidato tanto en híbrido como en `lexical_fallback`, con Chroma inaccesible en un router de prueba. No se interrumpió Chroma del entorno de demo.
- **Límite del mecanismo existente:** `reindex_chroma` hace upsert, no purga documentos ajenos a la base actual. La colección local contiene además documentos auxiliares/históricos sin fila en esta base, incluidos datos de pruebas. No se borraron. El matching filtra a IDs de PostgreSQL; estos documentos no se convierten en candidatos fantasma. El conteo relevante para esta verificación es 11 perfiles actuales, no el total bruto de la colección.
- PostgreSQL e índice no comparten transacción distribuida. Si falla la indexación después del commit, la respuesta indica reindexar; una recarga es segura.

## Validación

- Rust: `cargo fmt --check`, `SQLX_OFFLINE=true cargo check --locked`, `SQLX_OFFLINE=true cargo test --locked`: PASS; 74 pruebas. SQLX offline solo evita introspección de compilación; las pruebas de integración ejecutan PostgreSQL/Chroma locales.
- Prueba del seed reforzada: 20 fixtures, relaciones/constraints, 12 logs, recargas simultáneas, datos sin cambios, colisión tardía con rollback y recuperación, híbrido y fallback con varios candidatos.
- Frontend: `pnpm lint`, `pnpm exec tsc --noEmit`, `pnpm test`, `pnpm build`: PASS; 45 pruebas (una nueva del encuadre). No se actualizan métricas históricas de cobertura ni la presentación.
- Smoke HTTP autenticado: mapa, listas Empresa, catálogo ONG y envíos.
- Chromium 1440×1000: dashboards Empresa/ONG, catálogo, envíos y mapa PASS; 12 marcadores dentro del encuadre, sin marcador de usuario al denegar permiso y sin errores de consola.
- Capturas y resultados temporales de Chromium en `/tmp/ferxarp-demo-review/`; no se incorporan como evidencia histórica anterior.

Los registros son material de demostración. Los indicadores CEO calculados sobre ellos siguen siendo estimaciones sobre datos ficticios, no resultados de operación o impacto real.
