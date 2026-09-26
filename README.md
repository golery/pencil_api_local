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

## Binding

Listens on `127.0.0.1` and port `8558` (`PORT` or `--port`). The operating system then refuses connections from other machines.

`HOST` or `--host <addr>` opts in to another address, for example `--host 192.168.1.10` when testing from a phone. Use the address that device sends in the `Host` header. A request is rejected with `403` unless `Host` is `127.0.0.1:<port>`, `localhost:<port>`, or `<that address>:<port>`. That check blocks DNS rebinding.

## CORS and browser requests

Allows `https://pencil.golery.com` and local Next (`http://localhost:3000`, `http://127.0.0.1:3000`). Add more via `CORS_ORIGINS`. Includes `Access-Control-Allow-Private-Network: true`.

A request that sends `Origin` must use one of those origins, or the server returns `403`. Clients that omit `Origin`, such as `curl`, are allowed. A request with a body must use `Content-Type: application/json`; anything else returns `415`, so a page cannot post a plain-text body without a preflight.

Prefer local pencil_web (`http://localhost:3000`) when using vault books to avoid HTTPS→HTTP mixed content.

## Standalone CLI

`./scripts/publish.sh` builds a stripped binary for this machine. It does not need Rust installed to run:

```bash
./scripts/publish.sh
./dist/pencil-api-local
./dist/pencil-api-local --port 8558
```

`./dist/pencil-api-local --help` lists flags. `PORT`, `HOST`, `PENCIL_CONFIG`, `CORS_ORIGINS`, and a `.env` file in the working directory still apply.

Books live in `PENCIL_CONFIG`, or `~/.golery/pencil.json` when that variable is unset. The file looks like `{ "books": [] }`. On startup the server prints the config path. If the file is missing, it asks for the folder of the first book and creates the file.

## Release

TLDR: Just run `./scripts/release.sh`.
This will merge current branch to main brach and trigger the github action
for building the binary files and commit to /releases folder
[`.github/workflows/release.yml`](.github/workflows/release.yml)

## Publish

`./scripts/publish.sh` builds a binary for the machine you are on, under `dist/`. The release workflow runs the same script on Ubuntu and macOS.

## License

Copyright (c) 2026 Golery Inc. All rights reserved. You may clone, read, and build the original source. You may not modify it and build, and you may not redistribute it. See [LICENSE](LICENSE).

## Smoke test

```bash
cargo run
curl -X POST http://localhost:8558/api/pencil/book \
  -H 'Content-Type: application/json' \
  -d '{"name":"Personal","path":"./data/books/Personal"}'
curl http://localhost:8558/api/pencil/book
```
