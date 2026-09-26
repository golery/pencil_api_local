# pencil_api_local

Local vault-backed API for pencil_web. Runs on the user's laptop and serves registered markdown folders as Pencil books. Cloud books stay on goapi; the browser talks to this API directly at `http://127.0.0.1:8558`.

## Setup

Install Rust (https://rustup.rs), then:

```bash
cp .env.example .env
cargo run
```

From pencil_web, `npm run local-api:dev` starts this server.

## Endpoints

| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/health` | Liveness |
| GET | `/api/pencil/book` | Linked books from the config file |
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

`./scripts/release.sh` builds a stripped binary for this machine. It does not need Rust installed to run:

```bash
./scripts/release.sh
./dist/pencil-api-local
./dist/pencil-api-local --port 8558
```

`./dist/pencil-api-local --help` lists flags. `PORT`, `PENCIL_CONFIG`, `CORS_ORIGINS`, and a `.env` file in the working directory still apply.

Books live in `PENCIL_CONFIG`, or `~/.golery/pencil.json` when that variable is unset. The file looks like `{ "books": [] }`. On startup the server prints the config path. If the file is missing, it asks for the folder of the first book and creates the file.

## Release

Version lives in `Cargo.toml`. `./scripts/release.sh` writes `dist/pencil-api-local-<version>-<os>-<arch>` and `dist/SHA256SUMS`.

## Publish

`./scripts/publish.sh` builds that binary, copies it to `/home/hly/repos/releases/pencil_api_local`, then commits and pushes `golery/releases`. `git-lfs` must be on `PATH` (or in `~/.local/bin`) because that repo stores the binaries with Git LFS.

## Smoke test

```bash
cargo run
curl -X POST http://localhost:8558/api/pencil/book \
  -H 'Content-Type: application/json' \
  -d '{"name":"Personal","path":"./data/books/Personal"}'
curl http://localhost:8558/api/pencil/book
```
