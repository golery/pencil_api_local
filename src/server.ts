import { loadConfig } from "./config";
import { handleRequest } from "./routes/pencil";

const config = loadConfig();

const server = Bun.serve({
  port: config.port,
  async fetch(req) {
    return handleRequest(req, config);
  },
});

console.log(`pencil_api_local listening on http://localhost:${server.port}`);
console.log(`BOOKS_FILE=${config.booksFile}`);
console.log(`CORS origins: ${config.corsOrigins.join(", ")}`);
