# ACT-03: seguridad local con OWASP ZAP y pruebas dirigidas

## Entorno y alcance

Evaluación del 26-09-2026 sobre Compose **efímero** `ferxarp_act03`: PostgreSQL `127.0.0.1:15435` (base `ferxarp_act03`), ChromaDB `127.0.0.1:18003`, backend `http://127.0.0.1:18004`, frontend `http://localhost:13004`. Datos, JWT y Admin (`admin-act03@example.invalid`) exclusivos de prueba. Sin Supabase, Groq, Brevo ni Firebase. ZAP corrió en la red Docker **interna** `ferxarp_act03_scan`, con los únicos blancos `http://frontend:3000` y `http://backend:8000`.

El hook [`scripts/zap-scope.py`](../../../scripts/zap-scope.py) visitó `/`, `/login`, `/register`, `/dashboard`, `/shipments`, `/map`, `/health`, `/api/auth/login`, `/api/auth/register`, `/api/auth/me`, `/api/auth/ngos`, `/api/donations`, `/api/donations/feed`, `/api/donations/shipments`, `/api/scanner/map-points`, `/api/scanner/tracking/{id}` y `GET /api/seed/veracruz`. Se ejecutaron baseline (pasivo) y full scan (activo, límite de 8 minutos por blanco) antes y después. Esos scans fueron **anónimos**; no afirman cubrir formularios autenticados con ZAP. La API autenticada se evaluó aparte con solicitudes dirigidas y Admin de prueba. No se ejecutó la siembra ni fuzzing destructivo.

## Resultado ZAP

Conteo de **tipos de alerta por host** en los reportes JSON baseline; un tipo puede tener varias instancias. Los full scans confirmaron los mismos tipos y no agregaron otros. Los avisos de ZAP no detectaron el XSS almacenado; se identificó con el navegador y se informa aparte.

| Fase | High | Medium | Low | Informational |
|---|---:|---:|---:|---:|
| Antes | 0 | 3 | 3 | 1 |
| Después | 0 | 1 | 0 | 1 |

| ID | Hallazgo | Severidad | Endpoint | Antes | Corrección | Después | Estado final |
|---|---|---|---|---|---|---|---|
| SEC-01 | XSS almacenado en popup Leaflet | HIGH, true positive (manual complementaria) | `GET /api/scanner/map-points` → `/map` | Payload `<img src=x onerror=alert(1)>` almacenado y alerta `1` ejecutada en Chromium | Popup construido con nodos DOM y `textContent`; test Jest | Mismo payload permanece como texto; no alerta ni nodo `img` | Corregido |
| ZAP-10098 | Cross-Domain Misconfiguration, CORS `*` | MEDIUM, true positive | Backend `/health` y rutas API | `Access-Control-Allow-Origin: *` | `FRONTEND_ORIGIN` concreto, lista de orígenes y prueba de origen admitido/rechazado | 0 alertas; origen ajeno sin `Access-Control-Allow-Origin` | Corregido |
| ZAP-10020 | Falta protección clickjacking | MEDIUM, true positive | Frontend páginas HTML | 4 instancias baseline | `X-Frame-Options: DENY` | 0 alertas | Corregido |
| ZAP-10038 | Falta Content Security Policy | MEDIUM, true positive | Frontend páginas HTML | 5 instancias baseline | Sin cambio: una CSP de producción requiere integrar nonces con el HTML de Next y permitir de forma precisa Leaflet, teselas y API | 4 instancias baseline; 5 en active | **Pendiente P1** |
| ZAP-10021 | Falta `X-Content-Type-Options` | LOW, true positive | Frontend y backend `/health` | 5 + 1 instancias | `nosniff` en Next y Axum | 0 alertas | Corregido |
| ZAP-10037 | `X-Powered-By: Next.js` | LOW, true positive | Frontend | 5 instancias | `poweredByHeader: false` y copiar `next.config.ts` a la imagen runtime | 0 alertas | Corregido |
| ZAP-10109 | Modern Web Application | INFORMATIONAL, not applicable como vulnerabilidad | Frontend | 5 instancias | Ninguna | 5 instancias | Informativo |

Los JSON contienen URL, evidencia, confianza y detalle de cada instancia: [`zap-before-frontend.json`](zap-before-frontend.json), [`zap-before-backend.json`](zap-before-backend.json), [`zap-before-frontend-active.json`](zap-before-frontend-active.json), [`zap-before-backend-active.json`](zap-before-backend-active.json), y sus cuatro equivalentes `zap-after-*`. No se alteraron severidades. La opción `-I` del script solo evita que **warnings** impidan continuar; los reportes conservan todas las alertas.

## SQLi, XSS y autenticación dirigida

[`http-before.json`](http-before.json) y [`http-after.json`](http-after.json) registran método, ruta, payload y status sin JWT ni contraseña. En ambos pases:

| Input | Payload | Status después | Resultado |
|---|---|---:|---|
| `POST /api/auth/login` | `' OR '1'='1` | 401 | No entró como Admin |
| `POST /api/auth/register` correo | correo de prueba con `' OR '1'='1` | 201 | Quedó como dato literal; login del mismo correo 200 |
| `POST /api/donations` título | `' OR '1'='1` | 201 | Se recuperó el título literal (GET 200) |
| `GET /api/donations/{id}/matches` | ID `' OR '1'='1` | 400 | UUID inválido rechazado |
| `GET /api/donations/{id}/matches?include_ai=` | `' OR '1'='1` | 400 | Filtro inválido rechazado |
| `POST /api/scanner/scan` | UUID `' OR '1'='1` | 422 | Entrada inválida rechazada |
| `GET /api/scanner/tracking/{id}` | `' OR '1'='1` | 400 | UUID inválido rechazado |
| `POST /api/seed/veracruz` | Sin JWT | 401 | Sin mutación |

Esto no demuestra ausencia absoluta de SQLi; en los inputs probados no hubo bypass, alteración de consulta ni error SQL. Los controles autenticados usaron un JWT obtenido de login local y cubrieron donaciones, mapa, scanner y filtros. No hubo brute force. Para XSS, `POST /api/auth/register` almacenó literalmente el nombre ONG malicioso (201) y `GET /api/scanner/map-points` lo devolvió (200). En Chromium headless, [`browser-xss-before.json`](browser-xss-before.json) registra `alert_observed: true` y [`browser-xss-after.json`](browser-xss-after.json) registra `false`; la prueba automatizada del popup cubre nombre y detalles con HTML malicioso. Otros campos de donación, necesidades y `rejection_reason` se renderizan como texto React según inspección, pero no se sometieron todos a prueba de navegador dirigida: ese alcance queda explícitamente limitado.

## Reproducción

Con Docker, pnpm, Rust y las imágenes del proyecto disponibles:

```bash
umask 077
printf 'JWT_SECRET=act03-local-jwt-secret-only\n' > /tmp/ferxarp-act03-backend.env
export BACKEND_ENV_FILE=/tmp/ferxarp-act03-backend.env
export POSTGRES_DB=ferxarp_act03 POSTGRES_USER=ferxarp POSTGRES_PASSWORD=act03-local-only
export POSTGRES_PORT=15435 CHROMA_PORT=18003 BACKEND_PORT=18004 FRONTEND_PORT=13004
export FRONTEND_ORIGIN=http://localhost:13004
docker compose -p ferxarp_act03 build
docker compose -p ferxarp_act03 up -d --wait postgres chromadb
docker compose -p ferxarp_act03 run --rm --no-deps backend /app/migrate
FERXARP_ADMIN_EMAIL=admin-act03@example.invalid FERXARP_ADMIN_PASSWORD=act03-admin-password docker compose -p ferxarp_act03 run --rm --no-deps backend /app/provision_admin
docker compose -p ferxarp_act03 up -d --wait backend frontend
docker network create --internal ferxarp_act03_scan
docker network connect --alias frontend ferxarp_act03_scan ferxarp_act03-frontend-1
docker network connect --alias backend ferxarp_act03_scan ferxarp_act03-backend-1
scripts/security-scan.sh before
scripts/security-scan.sh after
```

Para reproducir las pruebas dirigidas, define `FERXARP_ADMIN_EMAIL` y `FERXARP_ADMIN_PASSWORD` **solo en el entorno**, y ejecuta `python3 scripts/security-http-check.py --phase after --sample-id nuevo --output docs/evidencias/security/http-after.json`. Para navegador, inicia ChromeDriver en `localhost:19515` y ejecuta `python3 scripts/browser-xss-check.py --phase after --output docs/evidencias/security/browser-xss-after.json`. Requieren solo URLs locales. Cuando termines: `docker compose -p ferxarp_act03 down -v --remove-orphans`, `docker network rm ferxarp_act03_scan` y elimina el archivo temporal. El script ZAP falla si la red no es interna. Los comandos `before` solo tienen sentido sobre la revisión anterior a las correcciones; los JSON versionados preservan esa medición.
