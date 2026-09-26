import pkg from "../package.json" with { type: "json" };
import { startServer } from "./server";

const HELP = `pencil-api-local ${pkg.version}

Start the local Pencil vault API.

Usage:
  pencil-api-local [options]

Options:
  -p, --port <port>         Listen port (default: 8300, or PORT)
  -b, --books-file <path>   Book registry JSON (default: ./data/books.json, or BOOKS_FILE)
  -h, --help                Show this help
  -v, --version             Show version

Environment:
  PORT            Listen port
  BOOKS_FILE      Book registry JSON
  CORS_ORIGINS    Extra allowed browser origins, comma-separated

A .env file in the working directory is loaded automatically.
`;

function fail(message: string): never {
  console.error(message);
  process.exit(1);
}

function takeValue(args: string[], index: number, flag: string): string {
  const value = args[index];
  if (!value || value.startsWith("-")) {
    fail(`Missing value for ${flag}`);
  }
  return value;
}

const args = process.argv.slice(2);
for (let i = 0; i < args.length; i++) {
  const arg = args[i]!;

  if (arg === "-h" || arg === "--help") {
    console.log(HELP);
    process.exit(0);
  }

  if (arg === "-v" || arg === "--version") {
    console.log(pkg.version);
    process.exit(0);
  }

  if (arg === "-p" || arg === "--port") {
    process.env.PORT = takeValue(args, ++i, arg);
    continue;
  }

  if (arg.startsWith("--port=")) {
    const value = arg.slice("--port=".length);
    if (!value) fail("Missing value for --port");
    process.env.PORT = value;
    continue;
  }

  if (arg === "-b" || arg === "--books-file") {
    process.env.BOOKS_FILE = takeValue(args, ++i, arg);
    continue;
  }

  if (arg.startsWith("--books-file=")) {
    const value = arg.slice("--books-file=".length);
    if (!value) fail("Missing value for --books-file");
    process.env.BOOKS_FILE = value;
    continue;
  }

  fail(`Unknown option: ${arg}\n\n${HELP}`);
}

startServer();
