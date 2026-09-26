# pencil_api_local

Local vault-backed API for pencil_web. Reads an Obsidian-style folder tree and exposes goapi-compatible pencil **read** endpoints (books + nodes). Coexists with goapi; use this instead of Postgres when working from local markdown files.

## Setup

```bash
bun install
cp .env.example .env
# edit VAULT_PATH if needed
bun run dev
```

Default: `http://localhost:8300`

## Endpoints

| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/pencil/book` | Books = top-level vault folders |
| GET | `/api/pencil/book/:bookId/node` | Flat node list (folders + `.md`) |
| POST | `/api/public/signin` | Stub `{ token: "local" }` |
| GET | `/api/user` | Stub local user |

Mutations return `501`. No auth enforcement (any Bearer / none accepted).

## CORS

Allows `https://pencil.golery.com` and local Next (`http://localhost:3000`, `http://127.0.0.1:3000`). Add more via `CORS_ORIGINS`. Preflight also sends `Access-Control-Allow-Private-Network: true` so Chrome can call localhost from the public site.

**Note:** An **https** page (`pencil.golery.com`) calling **http://localhost** is mixed content and many browsers block it. Prefer local pencil_web against this API, or expose the API over https (e.g. tunnel) if you need the production site to hit your machine.

## Vault mapping

- Top-level directories → books
- Subfolders and `.md` files → nodes (`text` = markdown body)
- Ignores `.obsidian`, `.git`, `node_modules`, other hidden entries, non-`.md` files

## Smoke test

```bash
VAULT_PATH=./fixtures/vault bun run start
curl http://localhost:8300/api/pencil/book
curl http://localhost:8300/api/pencil/book/<bookId>/node
```
