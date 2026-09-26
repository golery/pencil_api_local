# pencil_api_local

Local vault-backed API for pencil_web. Runs on the user's laptop and serves registered markdown folders as Pencil books. Cloud books stay on goapi; the browser talks to this API directly at `http://127.0.0.1:8300`.

## Setup

```bash
bun install
cp .env.example .env
bun run dev
```

## Endpoints

| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/health` | Liveness |
| GET | `/api/pencil/book` | Linked books from `data/books.json` |
| POST | `/api/pencil/book` | Body `{ name, path }` — link a folder |
| PATCH | `/api/pencil/book/:id` | Rename display name |
| DELETE | `/api/pencil/book/:id` | Unlink (files kept) |
| GET | `/api/pencil/book/:id/node` | Flat node list (folders + `.md`) |
| POST | `/api/public/signin` | Stub `{ token: "local" }` |
| GET | `/api/user` | Stub local user |

Node write mutations return `501`.

## CORS

Allows `https://pencil.golery.com` and local Next (`http://localhost:3000`, `http://127.0.0.1:3000`). Add more via `CORS_ORIGINS`. Includes `Access-Control-Allow-Private-Network: true`.

Prefer local pencil_web (`http://localhost:3000`) when using vault books to avoid HTTPS→HTTP mixed content.

## Standalone CLI

`bun run build` compiles a single binary for this machine. It does not need Bun installed to run:

```bash
bun run build
./dist/pencil-api-local
./dist/pencil-api-local --port 8300 --books-file ./data/books.json
```

`./dist/pencil-api-local --help` lists flags. `PORT`, `BOOKS_FILE`, `CORS_ORIGINS`, and a `.env` file in the working directory still apply.

## Release

Version lives in `package.json`. Pushing a matching `v*` tag builds binaries for Linux, macOS, and Windows and attaches them to a GitHub release.

```bash
# edit "version" in package.json, commit, then:
git tag v0.1.0
git push origin v0.1.0
```

To build every platform locally without publishing:

```bash
./scripts/release.sh
```

Artifacts land in `dist/` (`pencil-api-local-<version>-<os>-<arch>`, plus `SHA256SUMS`). `./scripts/release.sh --publish` uploads them when the current commit is already tagged `v<version>`.

## Smoke test

```bash
bun run start
curl -X POST http://localhost:8300/api/pencil/book \
  -H 'Content-Type: application/json' \
  -d '{"name":"Personal","path":"./fixtures/vault/Personal"}'
curl http://localhost:8300/api/pencil/book
```
