import type { Config } from "../config";
import {
  addBook,
  getBookRecord,
  loadRegistry,
  removeBook,
  updateBookName,
} from "../vault/registry";
import { listBooksFromRegistry, scanBookFolder, writeNodeText } from "../vault/scan";
import { stat } from "fs/promises";
import { resolve } from "path";

const NOT_IMPLEMENTED = { error: "Write operations are not supported by pencil_api_local" };

function corsHeaders(req: Request, config: Config): Record<string, string> {
  const origin = req.headers.get("Origin");
  const headers: Record<string, string> = {
    "Access-Control-Allow-Methods": "GET, POST, PUT, PATCH, DELETE, OPTIONS",
    "Access-Control-Allow-Headers": "Authorization, Content-Type, appId",
    "Access-Control-Max-Age": "86400",
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

function statusOf(err: unknown): number {
  if (err && typeof err === "object" && "status" in err && typeof (err as any).status === "number") {
    return (err as any).status;
  }
  return 500;
}

export async function handleRequest(req: Request, config: Config): Promise<Response> {
  if (req.method === "OPTIONS") {
    return corsPreflight(req, config);
  }

  const url = new URL(req.url);
  const path = url.pathname.replace(/\/+$/, "") || "/";

  try {
    if (path === "/api/health" && req.method === "GET") {
      return json({ ok: true }, req, config);
    }

    if (path === "/api/public/signin" && req.method === "POST") {
      return json({ token: "local" }, req, config);
    }

    if (path === "/api/user" && req.method === "GET") {
      return json({ id: 1, email: "local@pencil" }, req, config);
    }

    if (path === "/api/pencil/book" && req.method === "GET") {
      const records = await loadRegistry(config.booksFile);
      const books = await listBooksFromRegistry(records);
      return json(books, req, config);
    }

    if (path === "/api/pencil/book" && req.method === "POST") {
      const body = (await req.json().catch(() => ({}))) as { name?: string; path?: string };
      const name = body.name?.trim();
      const folderPath = body.path?.trim();
      if (!name || !folderPath) {
        return json({ error: "name and path are required" }, req, config, 400);
      }
      const abs = resolve(folderPath);
      const st = await stat(abs).catch(() => null);
      if (!st || !st.isDirectory()) {
        return json({ error: `Not a directory: ${abs}` }, req, config, 400);
      }
      const record = await addBook(config.booksFile, name, abs);
      const { book } = await scanBookFolder(record);
      return json({ book }, req, config, 201);
    }

    const bookIdMatch = path.match(/^\/api\/pencil\/book\/(\d+)$/);
    if (bookIdMatch) {
      const bookId = Number(bookIdMatch[1]);
      if (req.method === "PATCH") {
        const body = (await req.json().catch(() => ({}))) as { name?: string };
        const name = body.name?.trim();
        if (!name) {
          return json({ error: "name is required" }, req, config, 400);
        }
        const record = await updateBookName(config.booksFile, bookId, name);
        const { book } = await scanBookFolder(record).catch(async () => {
          return {
            book: {
              id: record.id,
              code: record.name,
              rootId: 0,
              name: record.name,
              order: record.order,
              userId: "local",
              folderPath: record.path,
            },
          };
        });
        return json(book, req, config);
      }
      if (req.method === "DELETE") {
        await removeBook(config.booksFile, bookId);
        return json({ ok: true }, req, config);
      }
    }

    const bookNodesMatch = path.match(/^\/api\/pencil\/book\/(\d+)\/node$/);
    if (bookNodesMatch && req.method === "GET") {
      const bookId = Number(bookNodesMatch[1]);
      const record = await getBookRecord(config.booksFile, bookId);
      if (!record) {
        return json({ error: "Book not found" }, req, config, 404);
      }
      const { nodes } = await scanBookFolder(record);
      return json(nodes, req, config);
    }

    const writeNodeMatch = path.match(/^\/api\/pencil\/book\/(\d+)\/node\/(\d+)$/);
    if (writeNodeMatch && req.method === "PUT") {
      const bookId = Number(writeNodeMatch[1]);
      const nodeId = Number(writeNodeMatch[2]);
      const body = (await req.json().catch(() => ({}))) as { text?: string };
      if (typeof body.text !== "string") {
        return json({ error: "text (string) is required" }, req, config, 400);
      }
      const record = await getBookRecord(config.booksFile, bookId);
      if (!record) {
        return json({ error: "Book not found" }, req, config, 404);
      }
      const node = await writeNodeText(record, nodeId, body.text);
      return json(node, req, config);
    }

    // Remaining goapi mutation routes → 501
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
    return json({ error: message }, req, config, statusOf(err));
  }
}
