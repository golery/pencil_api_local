import type { Config } from "../config";
import { scanVault } from "../vault/scan";

const NOT_IMPLEMENTED = { error: "Write operations are not supported by pencil_api_local" };

function corsHeaders(req: Request, config: Config): Record<string, string> {
  const origin = req.headers.get("Origin");
  const headers: Record<string, string> = {
    "Access-Control-Allow-Methods": "GET, POST, PUT, PATCH, DELETE, OPTIONS",
    "Access-Control-Allow-Headers": "Authorization, Content-Type, appId",
    "Access-Control-Max-Age": "86400",
    // Chrome Private Network Access: public site (pencil.golery.com) → localhost
    "Access-Control-Allow-Private-Network": "true",
  };

  if (origin && config.corsOrigins.includes(origin)) {
    headers["Access-Control-Allow-Origin"] = origin;
    headers["Access-Control-Allow-Credentials"] = "true";
    headers["Vary"] = "Origin";
  }

  return headers;
}

function json(data: unknown, req: Request, config: Config, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json",
      ...corsHeaders(req, config),
    },
  });
}

function corsPreflight(req: Request, config: Config): Response {
  return new Response(null, {
    status: 204,
    headers: corsHeaders(req, config),
  });
}

export async function handleRequest(req: Request, config: Config): Promise<Response> {
  if (req.method === "OPTIONS") {
    return corsPreflight(req, config);
  }

  const url = new URL(req.url);
  const path = url.pathname.replace(/\/+$/, "") || "/";

  try {
    if (path === "/api/public/signin" && req.method === "POST") {
      return json({ token: "local" }, req, config);
    }

    if (path === "/api/user" && req.method === "GET") {
      return json({ id: 1, email: "local@pencil" }, req, config);
    }

    if (path === "/api/pencil/book" && req.method === "GET") {
      const { books } = await scanVault(config.vaultPath);
      return json(books, req, config);
    }

    const bookNodesMatch = path.match(/^\/api\/pencil\/book\/(\d+)\/node$/);
    if (bookNodesMatch && req.method === "GET") {
      const bookId = Number(bookNodesMatch[1]);
      const { nodesByBookId } = await scanVault(config.vaultPath);
      const nodes = nodesByBookId.get(bookId);
      if (!nodes) {
        return json({ error: "Book not found" }, req, config, 404);
      }
      return json(nodes, req, config);
    }

    // Known goapi mutation routes → 501
    if (
      path === "/api/pencil/book" &&
      (req.method === "POST" || req.method === "PUT" || req.method === "PATCH" || req.method === "DELETE")
    ) {
      return json(NOT_IMPLEMENTED, req, config, 501);
    }
    if (
      /^\/api\/pencil\/book\/\d+$/.test(path) &&
      (req.method === "PATCH" || req.method === "DELETE" || req.method === "PUT")
    ) {
      return json(NOT_IMPLEMENTED, req, config, 501);
    }
    if (
      path === "/api/pencil/book/reorder" ||
      /^\/api\/pencil\/(move|add|delete)\/\d+$/.test(path) ||
      path === "/api/pencil/update"
    ) {
      if (req.method !== "GET") {
        return json(NOT_IMPLEMENTED, req, config, 501);
      }
    }

    return json({ error: `Not found: ${req.method} ${path}` }, req, config, 404);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error(err);
    return json({ error: message }, req, config, 500);
  }
}
