# FERXARP en Eisenherz

Despliegue exclusivo del usuario `ferxarp-deploy`, mediante `ssh ferxarp-eisenherz`, en `/srv/ferxarp`. Cada operación Docker pasa por `compose.sh`, que exige el usuario/host correctos, contexto `rootless`, ausencia de `DOCKER_HOST` y SecurityOptions rootless. No utiliza Docker del sistema.

- `/srv/ferxarp/repo`: archivo Git de la revisión FERXARP desplegada.
- `/srv/ferxarp/presentation`: archivo Git de la presentación, sin cambios de contenido.
- `/srv/ferxarp/deploy/.env`: configuración privada, modo 0600; nunca se copia desde desarrollo.
- `/srv/ferxarp/deploy/source.json`: SHAs y SHA256 de los archivos transferidos.
- `/srv/ferxarp/data` y `/srv/ferxarp/backups`: reservados para FERXARP.
- Volúmenes rootless: `ferxarp_postgres_data`, `ferxarp_chroma_data`.

Servicios: frontend, backend, postgres, chroma y presentation. Bases exclusivamente en red interna; backend sin publicación de puertos. Frontend publica `127.0.0.1:18080`; presentación `127.0.0.1:18081`. El frontend usa API del mismo origen y proxy interno Next hacia backend. Los secretos solo se inyectan al servicio que los requiere.

Destinos públicos autorizados:

- `https://ferxarp.lobelisque.space` → `http://127.0.0.1:18080`
- `https://ferxarp-slides.lobelisque.space` → `http://127.0.0.1:18081`

La publicación requiere Cloudflare Tunnel persistente y autorización de la cuenta. No usar Quick Tunnel; no cambiar registros DNS ajenos ni abrir puertos del router. El conector se configura exclusivamente para FERXARP, una vez autorizado, con credenciales privadas fuera de Git.

Comandos remotos (desde SSH):

```bash
bash /srv/ferxarp/repo/deploy/eisenherz/compose.sh build
bash /srv/ferxarp/repo/deploy/eisenherz/compose.sh up -d postgres chroma --wait
bash /srv/ferxarp/repo/deploy/eisenherz/compose.sh run --rm --no-deps backend /app/migrate
# Provisionar con el binario oficial y variables privadas, sin imprimirlas.
bash /srv/ferxarp/repo/deploy/eisenherz/compose.sh up -d --wait
python3 /srv/ferxarp/repo/deploy/eisenherz/initialize.py
```

`initialize.py` provisiona Admin con el mecanismo oficial, inicia sesión por el proxy y carga dos veces el endpoint oficial de seed. Compara los conteos de PostgreSQL con las cantidades leídas del fixture Rust y las huellas de tablas antes/después de la segunda carga. No contiene contraseñas ni crea fixtures alternativos. Solo debe usarse para la instalación demo autorizada; no elimina datos.

La presentación se construye con `NEXT_PUBLIC_FERXARP_DEMO_URL` apuntando al dominio de la aplicación; su fuente permanece intacta. Las imágenes llevan etiquetas de SHA; el SHA de la presentación y sus argumentos de build quedan en el registro de despliegue.
