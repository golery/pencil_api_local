# Fix: lock pencil_api_local to this machine and to Pencil

Status: done. The server binds to `127.0.0.1` by default, rejects a foreign `Origin`, requires `Content-Type: application/json` on requests with a body, and rejects a `Host` that is not loopback or the `--host` address.

The server has no authentication and can read any markdown under a linked folder, and `POST /api/pencil/book` links any directory. Three gaps let someone other than the user reach it.

## 1. Server listens on every network interface

`src/lib.rs`:

```rust
let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
```

`0.0.0.0` accepts connections from any device on the same network (café, office Wi-Fi). CORS does not help: only browsers enforce it, `curl` ignores it. Someone on the network can:

- `GET /api/pencil/book/:id/node` to download the full text of every `.md` in a linked folder.
- `POST /api/pencil/book` with `{"name":"x","path":"/home/<user>"}`, then read every markdown file under that directory.

**Fix**

- Bind to `127.0.0.1`. The OS then refuses connections from other machines.
- Add an opt-in `--host <addr>` flag (and `HOST` env var) for anyone who needs LAN access, e.g. testing from a phone. Default stays `127.0.0.1`.
- Update the startup log and `--help`.

**Check**: on WSL2, confirm a browser on Windows still reaches `http://localhost:8558` after the change (WSL forwards `localhost`, but verify).

## 2. Requests from any website are executed

CORS stops other origins from *reading* responses, but `handle` in `src/http.rs` still runs every request. `read_json` parses the body whatever the `Content-Type` is:

```rust
match serde_json::from_slice::<Value>(&bytes) {
    Ok(Value::Object(map)) => Ok(map),
    _ => Ok(Map::new()),
}
```

So any page the user visits can do:

```js
fetch('http://127.0.0.1:8558/api/pencil/book', {
  method: 'POST',
  body: '{"name":"x","path":"/"}', // sent as text/plain: no preflight
});
```

The browser sends it without a preflight and the folder gets linked. The attacker can't read the response, but can change the registry. Once node writes are implemented (they return `501` today), this would let a website modify files.

**Fix** (in `handle`, before `dispatch`):

- If an `Origin` header is present and not in `config.cors_origins`, return `403`. Requests without `Origin` (curl, the CLI) stay allowed. That's fine once the server only listens on `127.0.0.1`.
- For requests with a body, require `Content-Type: application/json`, otherwise return `415`. This forces a browser preflight, which the origin allowlist then blocks.

## 3. DNS rebinding

An attacker makes `evil.com` resolve to `127.0.0.1` after the page loads. The page's requests are then same-origin to the browser, so CORS doesn't apply and full responses are readable. Fix 1 doesn't stop this, because the request really does come from the user's machine.

**Fix**: in `handle`, reject (`403`) any request whose `Host` header is not one of:

- `127.0.0.1:<port>`
- `localhost:<port>`
- plus whatever `--host` was set to, if the user opted in

## Summary

| Gap | Who can exploit it | Fix |
|---|---|---|
| Listening on all interfaces | Anyone on the same network | Listen on `127.0.0.1`; `--host` to opt in to LAN |
| Requests from any site are executed | Any website the user visits | Enforce the `Origin` allowlist; require JSON `Content-Type` |
| DNS rebinding | A crafted website | Check the `Host` header |

## Tests to add

- Request with a disallowed `Origin` returns `403`; allowed origin and no origin return `200`.
- `POST` with `Content-Type: text/plain` returns `415`.
- `Host: evil.com:8558` returns `403`; `Host: localhost:8558` returns `200`.
- `OPTIONS` preflight from a disallowed origin gets no `Access-Control-Allow-Origin` header.

## Later (optional)

A pairing token: on first run the server prints a one-time code, the user enters it in Pencil once, and the browser sends it on every request. It's stronger than origin checks, but it adds a setup step. Revisit when node writes land.

## After the fix

- README: document `--host`, the origin and host checks, and the `127.0.0.1` default.
- pencil_web explainer page: the "Only this computer can connect" and "Only Pencil's website can use it" claims become accurate.
