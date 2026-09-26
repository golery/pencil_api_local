import { loadConfig, type Config } from "./config";
import { handleRequest } from "./routes/pencil";

export function startServer(config: Config = loadConfig()) {
  const server = Bun.serve({
    port: config.port,
    async fetch(req) {
      return handleRequest(req, config);
    },
  });

  console.log(`pencil_api_local listening on http://localhost:${server.port}`);
  console.log(`BOOKS_FILE=${config.booksFile}`);
  console.log(`CORS origins: ${config.corsOrigins.join(", ")}`);
  return server;
}

if (import.meta.main) {
  startServer();
}
