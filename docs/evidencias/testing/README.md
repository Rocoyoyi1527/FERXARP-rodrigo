# ACT-01 — Pruebas y cobertura

Fecha: 2026-09-25. Base: `ea2673a1dd8eddaaf4ded0e063a388fa87a226e4` en `feat/activity-01-testing-coverage`. Árbol inicial limpio.

## Resultado

| Evidencia | Anterior | ACT-01 | Resultado |
| --- | ---: | ---: | --- |
| Tests Rust | 71 | 73 (+2) | PASS |
| Tests Jest | 0 | 35 | PASS |
| Rust completo, líneas | 68.65% | 75.00% | Referencia sin exclusiones |
| Rust módulo escolar, líneas | 77.52% | **85.91%** | Supera 80% |
| Rust módulo escolar, funciones | — | 70.81% | Reportado sin ocultar |
| Rust módulo escolar, regiones | — | 84.20% | Reportado sin ocultar |
| Jest, instrucciones | — | **97.79%** | Supera 80% |
| Jest, ramas | — | **91.42%** | Supera 80% |
| Jest, funciones | — | **93.18%** | Supera 80% |
| Jest, líneas | — | **97.79%** | Supera 80% |

El criterio de cobertura Rust se evalúa en **líneas del módulo escolar**, conforme al objetivo ACT-01. Las funciones Rust no alcanzan 80%; el porcentaje global sin exclusiones tampoco. Ambos quedan explícitos para no confundirlos con la cobertura del módulo. El informe de Jest conserva [el resumen JSON por archivo](jest-coverage-summary.json).

## Alcance y justificación

Rust incluye `admin_bootstrap.rs`, `models/donation_state.rs`, `api/{auth,middleware,donations,scanner}.rs` y `ai/{chroma_db,embedding,index,matcher,scoring}.rs`. Esto cubre JWT, roles, ownership, bootstrap, lifecycle, ChromaDB y matching puro. El reporte completo mide adicionalmente todo lo instrumentado. El reporte del módulo excluye `api/metrics.rs` (métricas CEO fuera del criterio 1), `api/seed.rs` (datos de demostración), `ai/groq.rs` (servicio externo; no se invoca Groq real), `src/bin/*` y `src/main.rs` (entrypoints y wiring), y código instrumentado de `std` del compilador. No se excluyen líneas ni funciones dentro de los archivos incluidos. `api/scanner.rs` queda en 61.54% de líneas y se mantiene visible en el resumen generado.

Jest mide exactamente:

- `frontend/src/lib/api.ts`
- `frontend/src/app/(auth)/login/page.tsx`
- `frontend/src/app/(auth)/register/page.tsx`
- `frontend/src/app/shipments/page.tsx`
- `frontend/src/components/scanner/StockScanner.tsx`

Los tests Jest ejercitan login, registro y roles de registro, token en requests, estados HTTP 401/403/409, permisos y acciones por etapa, rechazo con motivo y navegación de sesión inválida. Usan `fetch` o funciones API simuladas; no requieren backend real. Los dos tests Rust nuevos comprueban aislamiento del inventario/feed y visibilidad de envíos por rol con PostgreSQL local. La suite Rust anterior de 71 tests se conserva.

## Reproducción

Requiere PostgreSQL y ChromaDB locales sanos, migraciones aplicadas a `backend/.env`, `cargo-llvm-cov`, herramientas LLVM y SQLx CLI. Las instrucciones del entorno están en el [README principal](../../../README.md#pruebas-y-cobertura). En este host Arch sin `rustup` se usaron `LLVM_COV=/usr/bin/llvm-cov`, `LLVM_PROFDATA=/usr/bin/llvm-profdata` y `SQLX_OFFLINE=true` para compilar las pruebas; SQLx `prepare --check` se ejecutó online tras aplicar la migración local pendiente.

```bash
cd backend
cargo test --locked
mkdir -p coverage
cargo llvm-cov --all-features --workspace --lcov --output-path coverage/lcov.info
./scripts/coverage.sh
cargo sqlx prepare --check -- --locked

cd ../frontend
pnpm test
pnpm test:coverage
```

La salida completa queda en `backend/coverage/lcov.info` y `backend/coverage/{full,module}-summary.txt`; el resumen de Jest se regenera en `frontend/coverage/coverage-summary.json`. Ambos directorios están ignorados por Git. No se versiona HTML de cobertura.

## Validación ACT-01

Backend: `cargo fmt --check`, `cargo check --locked`, `cargo test --locked`, `cargo clippy --all-targets --all-features -- -D warnings` y `cargo sqlx prepare --check -- --locked`: **PASS**. Frontend: `pnpm lint`, `pnpm exec tsc --noEmit`, `pnpm test`, `pnpm test:coverage` y `pnpm build`: **PASS**. El build usa webpack y la API TypeScript de Next para evitar fallos del CLI/Turbopack en el entorno Node 26 usado para esta medición.

El módulo funcional está evidenciado por tests HTTP de extremo a extremo internos del backend, lifecycle transaccional y build del frontend. La autenticación JWT y los cuatro roles tienen tests Rust de acceso y tests Jest del flujo visible. No se asigna una calificación.
