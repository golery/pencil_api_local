import { resolve } from "path";

const DEFAULT_CORS_ORIGINS = [
  "https://pencil.golery.com",
  "http://localhost:3000",
  "http://127.0.0.1:3000",
];

export type Config = {
  booksFile: string;
  port: number;
  corsOrigins: string[];
};

export function loadConfig(): Config {
  const port = Number(process.env.PORT || "8300");
  if (!Number.isFinite(port) || port <= 0) {
    throw new Error(`Invalid PORT: ${process.env.PORT}`);
  }

  const booksFile = resolve(process.env.BOOKS_FILE || "./data/books.json");

  const extra = (process.env.CORS_ORIGINS || "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);

  return {
    booksFile,
    port,
    corsOrigins: [...new Set([...DEFAULT_CORS_ORIGINS, ...extra])],
  };
}
