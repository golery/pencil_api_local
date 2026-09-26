import { resolve } from "path";

const DEFAULT_CORS_ORIGINS = [
  "https://pencil.golery.com",
  "http://localhost:3000",
  "http://127.0.0.1:3000",
];

export type Config = {
  vaultPath: string;
  port: number;
  corsOrigins: string[];
};

export function loadConfig(): Config {
  const vaultPathRaw = process.env.VAULT_PATH;
  if (!vaultPathRaw) {
    throw new Error("VAULT_PATH is required (see .env.example)");
  }

  const port = Number(process.env.PORT || "8300");
  if (!Number.isFinite(port) || port <= 0) {
    throw new Error(`Invalid PORT: ${process.env.PORT}`);
  }

  const extra = (process.env.CORS_ORIGINS || "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);

  return {
    vaultPath: resolve(vaultPathRaw),
    port,
    corsOrigins: [...new Set([...DEFAULT_CORS_ORIGINS, ...extra])],
  };
}
