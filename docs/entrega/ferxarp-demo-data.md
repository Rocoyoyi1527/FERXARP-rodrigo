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


## FERXARP-DEMO-CLEANUP · 30 de septiembre de 2026

### Pre-flight y alcance

Directorio exclusivo: `FERXARP`; rama `feat/demo-data-veracruz`; HEAD inicial `4d05572`. `pwd`, `git branch --show-current`, `git status --short` y `git diff --check`: correctos, árbol inicialmente limpio. La ampliación ya estaba versionada antes de esta tarea. No se modifica `FERXARP_presentacion_WI`; no se realiza commit, push ni despliegue remoto.

Se conserva el seed completo y la base local. No hay migraciones, nuevos registros, cambios de coordenadas, matching, lifecycle, permisos, autenticación o CI/CD. En archivos de autenticación solo se corrige un comentario que nombraba un proveedor anterior. Los cambios Rust restantes son mensajes de conexión; no cambian el contrato del healthcheck.

### Presentación e identificación

Un aviso verde discreto por vista informa: **Entorno de demostración. Las organizaciones, donaciones y operaciones mostradas como Demo utilizan datos ficticios.** Se utiliza en dashboards ONG/Empresa/Admin, solicitudes/envíos y mapa cuando contienen fixtures; Dirección lo muestra al detectar ONG del seed y mantiene las métricas calificadas como estimaciones.

Las tarjetas muestran badge **Demo**, título sin prefijo repetido y descripción funcional. Ejemplo: **Demo · Leche sellada** más advertencia larga → **[Demo] Leche sellada**, «Leche para comedores; cantidad expresada en cajas.», «Empresa: Alimentos Veracruz Demo», 80 unidades. Las advertencias originales permanecen almacenadas en PostgreSQL y en el seed para trazabilidad; solo cambia su representación visual. El Admin conserva correos como evidencia secundaria y un aviso de verificación simulada y coordenadas aproximadas. Los motivos de rechazo simulados se conservan.

No existe un campo explícito demo ni nombre/razón social de Empresa en el modelo/API actual. `frontend/src/lib/demo.ts` reconoce únicamente los UUID exactos del seed: namespace `fea00000-0000-4000-8000`, tipo 4 con índices 0–19 para donaciones y tipo 3 con índices 0–9 para ONG. No identifica por título, prefijo amplio ni dominio. Una prueba contrasta el namespace y el número de fixtures con el código Rust; otra conserva textos de registros históricos aunque parezcan demo.

Las seis etiquetas de Empresa se derivan exclusivamente de los identificadores de cuenta del seed: Alimentos Veracruz Demo, Tecnología Boca Demo, Mobiliario Centro Demo, Suministros Medellín Demo, Herramientas Puerto Demo y Textiles Veracruz Demo. Son etiquetas de presentación, no razones sociales obtenidas del API. Solo se sustituyen los seis correos exactos al presentar una donación con UUID demo conocido. Una cuenta o donación ajena conserva su identificador original.

### Registro histórico y conservación

«Cajas de leche», UUID `f5d13322-a795-4ef3-bccc-a36771bd9c07`, pertenece al usuario Empresa `233f7148-b265-4ed2-bf5f-f745d3db0184` (cuenta local `orga@hotmail.com`, ajena al seed). Creación: 27 de septiembre de 2026, 00:51:18 UTC. Bitácora: salida 00:57:17 UTC y entrega 00:57:22 UTC, ambas sin notas. Estos hechos prueban que es anterior al nuevo dataset, pero no prueban que sea una cuenta de pruebas ni que surgiera de un smoke autorizado. No se borra, modifica, oculta ni marca como demo. No se propone una eliminación sin esa evidencia.

El recorrido de navegador compara huellas de todas las filas de `users`, `ngos`, `donations`, `donation_requests` y `delivery_logs` antes y después: sin cambios. Las pruebas Rust usan su base aislada habitual.

### Mapa y contadores

Encuadre, coordenadas, marcadores, geolocalización, conexiones y leyenda sin cambios. Se agrega el aviso global y se simplifica el popup demo, manteniendo su identificación. Los 12 marcadores quedan dentro del encuadre; ONG verdes, almacén diferenciado, sin marcador personal cuando se deniega permiso. Se conserva la leyenda de conexiones de referencia y ausencia de seguimiento GPS. No se incorpora clustering. La ONG histórica del centro comparte ubicación con el almacén de referencia; se conserva esa coordenada existente.

Todos los contadores se calculan desde las respuestas del API; no se agregan números fijos a la UI. Verificados contra Chromium:

| Vista local / cuenta principal | Conteos |
|---|---|
| ONG Inicio | 8 disponibles, 2 solicitudes reservadas (1 pendiente + 1 aprobada), 2 en camino, 1 recibida |
| Empresa Inicio | 6 activas, 1 pendiente, 1 lista para salida, 1 entregada |
| Solicitudes ONG/Empresa | 1 pendiente, 1 aprobada, 2 en camino, 2 finalizadas (1 entrega + 1 rechazo) |
| Mapa ONG/Empresa | 11 ONG, 1 almacén, 2 envíos activos visibles para esa cuenta |
| Mapa Admin | 11 ONG, 1 almacén, 4 envíos activos |
| Admin ONG | 11 registradas: 10 verificadas (incluye histórica) y 1 pendiente |
| Base local | 21 donaciones: 8 acopio, 4 reservadas, 4 tránsito, 3 entregadas, 2 rechazadas; 13 solicitudes |

### Admin e idempotencia

«Cargar datos demo» continúa exclusivo de Admin. El modal dice: «Se crearán o actualizarán datos ficticios para demostración. Los registros existentes se conservan.» Se abrió y canceló en Chromium; no se recargó el seed de la base local. La prueba Rust existente verifica recarga/concurrencia, conservación de estados y colisiones con rollback, y vuelve a pasar.

### Instalación nueva recomendada

Ejecutar en un destino nuevo con volúmenes PostgreSQL **y Chroma** nuevos, sin restaurar backups ni montar volúmenes del ensayo local. No usar una operación destructiva de limpieza sobre la instalación existente. El nombre de proyecto siguiente debe estar libre; los puertos configurados también.

Preparar fuera de Git un archivo privado de entorno del backend con `JWT_SECRET` y `FERXARP_SEED_PASSWORD` propios. Definir `POSTGRES_PASSWORD` de forma privada para Compose. No usar las credenciales de ensayo como credenciales finales. `BACKEND_ENV_FILE` apunta a ese archivo; no se publican secretos en este procedimiento.

Desde la raíz de FERXARP:

```bash
export COMPOSE_PROJECT_NAME=ferxarp-demo-clean
export BACKEND_ENV_FILE=/ruta/privada/ferxarp-demo.env
# POSTGRES_PASSWORD debe estar configurada en el entorno privado.
```

1. **Levantar PostgreSQL e índice nuevo.**

   ```bash
   docker compose up -d postgres chromadb
   docker compose ps
   ```

   Esperar ambos servicios saludables. No reutilizar la colección histórica local: reindexar hace upsert y no elimina documentos ajenos automáticamente.

2. **Compilar y ejecutar migraciones existentes.**

   ```bash
   docker compose build backend frontend
   docker compose run --rm --no-deps backend /app/migrate
   ```

3. **Provisionar Admin con el binario existente.**

   ```bash
   read -r -p 'Correo Admin: ' FERXARP_ADMIN_EMAIL
   read -r -s -p 'Contraseña Admin (mínimo 12 caracteres): ' FERXARP_ADMIN_PASSWORD
   printf '\n'
   export FERXARP_ADMIN_EMAIL FERXARP_ADMIN_PASSWORD
   docker compose run --rm --no-deps -e FERXARP_ADMIN_EMAIL -e FERXARP_ADMIN_PASSWORD backend /app/provision_admin
   unset FERXARP_ADMIN_EMAIL FERXARP_ADMIN_PASSWORD
   docker compose up -d backend frontend
   ```

4. **Cargar el seed actual.** Iniciar sesión como ese Admin y usar «Cargar datos demo». Alternativa: `POST /api/seed/veracruz?include_ai=false` con JWT Admin. Esperar resultado de 6 empresas, 10 ONG y 20 donaciones. No registrar cuentas adicionales durante esta validación. Una segunda carga conserva el dataset y sus progresos; no es reset.

5. **Reindexar Chroma si corresponde.** El seed ya indexa las ONG. Ante una respuesta de indexación incompleta, configurar `DATABASE_URL` y `CHROMA_URL` para ese mismo entorno y ejecutar desde `backend/`:

   ```bash
   SQLX_OFFLINE=true cargo run --locked --bin reindex_chroma
   ```

   El binario de reindexación se ejecuta desde el código fuente; la imagen Docker actual solo empaqueta backend, migraciones y provisionamiento Admin. En la instalación limpia deben procesarse 10 ONG, sin fallos. No apuntar por accidente a la base del ensayo anterior.

6. **Validar salud.** Con los puertos por defecto:

   ```bash
   curl --fail http://localhost:8000/health
   curl --fail http://localhost:8001/api/v2/heartbeat
   curl --fail --output /dev/null http://localhost:3000
   docker compose ps
   ```

   Deben responder HTTP 200. Adaptar URLs si se configuraron otros puertos.

7. **Validar conteos antes de operar.**

   ```bash
   docker compose exec -T postgres psql -U ferxarp -d ferxarp -c "SELECT role,count(*) FROM users GROUP BY role; SELECT count(*) AS ngos FROM ngos; SELECT status,count(*) FROM donations GROUP BY status; SELECT count(*) AS solicitudes FROM donation_requests; SELECT count(*) AS logs FROM delivery_logs; SELECT count(*) AS historico FROM donations WHERE id='f5d13322-a795-4ef3-bccc-a36771bd9c07';"
   ```

   Ajustar usuario/base si se personalizaron. Valores esperados: **1 Admin, 6 Empresa, 10 ONG** (17 usuarios), **10 perfiles ONG** (9 verificados simulados, 1 pendiente), **20 donaciones** (8 acopio, 4 reservadas, 4 tránsito, 2 entregadas, 2 rechazadas), **12 solicitudes**, **12 logs**, **0 registros con el UUID histórico**. El mapa mostrará **10 ONG + 1 almacén de referencia**. Las 11 ONG del entorno local incluyen una histórica y no son el conteo de una instalación nueva. No se agrega una ONG para igualar ese histórico.

Este procedimiento se documenta para el despliegue posterior; no se ejecutó un despliegue nuevo durante cleanup. La reproducibilidad e idempotencia del seed se verifican en la suite Rust aislada.

### Evidencia de cleanup

- Rust: `cargo fmt --check`, `SQLX_OFFLINE=true cargo check --locked`, `cargo test --locked`: PASS, 74 pruebas.
- Frontend: `pnpm lint`, `pnpm exec tsc --noEmit`, `pnpm test`, `pnpm build`: PASS, 49 pruebas (45 existentes + 4 regresiones de identidad/presentación).
- Frontend Docker reconstruido y arrancado en el entorno local para validar los cambios. El backend local conserva la imagen anterior; los únicos cambios de fuente backend son comentarios y mensajes de proveedor, verificados por compilación/pruebas.
- Chromium en anchos 1440 y 390 px: ONG Inicio/catálogo/envíos/mapa; Empresa Inicio/publicaciones/envíos; Admin ONG/modal demo. Sin overflow ni errores de consola; conteos DOM contrastados con API. Menú móvil Admin mantiene la acción exclusiva.
- Empresa/ONG mediante login normal del seed. Admin mediante JWT temporal firmado localmente para el usuario Admin existente, sin conocer/cambiar contraseña ni crear cuentas. Esto no constituye una nueva prueba de login Admin por contraseña.
- [Resultados de navegador](../evidencias/demo-cleanup/results.json), [Dashboard ONG](../evidencias/demo-cleanup/ong-dashboard.png), [Catálogo ONG](../evidencias/demo-cleanup/ong-catalog.png), [Mapa](../evidencias/demo-cleanup/map.png), [Dashboard Empresa](../evidencias/demo-cleanup/empresa-dashboard.png). La carpeta incluye capturas adicionales de envíos, Admin y móvil.

### Auditoría textual final

Se ejecutó la búsqueda solicitada en backend, frontend, README y este documento. Cada coincidencia queda clasificada con ubicación y texto en [auditoria-textual.tsv](../evidencias/demo-cleanup/auditoria-textual.tsv).

- **SIMPLIFICAR → resuelto:** advertencias largas repetidas en tarjetas y popups demo; se conservan como evidencia almacenada y se presentan mediante aviso global + badge. Correos de empresas demo sustituidos por sus etiquetas de cuenta en tarjetas. La descripción de métricas en README ahora explicita estimaciones.
- **ELIMINAR → resuelto:** referencias obsoletas al proveedor de base de datos en README, mensajes de arranque/raíz y comentarios de registro, sustituidas por PostgreSQL.
- **CORRECTA:** correos reservados del seed y documentación de acceso; advertencias de fixtures/logs; funciones de formato y regresiones; nombres de campos de estimación; disclaimer CEO; referencias al modelo Groq que efectivamente configura el código (sin llamada a Groq en esta tarea); coincidencia incidental en checksum del lockfile. No se alteran dependencias.

No quedan bloqueos de cleanup. Se conserva el histórico local y las limitaciones metodológicas de las estimaciones; para el despliegue se requieren volúmenes nuevos y configuración privada del destino. No se afirma que el despliegue remoto se haya ejecutado.
