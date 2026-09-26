import { readdir, readFile, stat, writeFile } from "fs/promises";
import { basename, join, relative, resolve, sep } from "path";
import { pathToId } from "./ids";
import type { BookRecord } from "./registry";

const IGNORED_DIR_NAMES = new Set([".obsidian", ".git", "node_modules"]);

export type Book = {
  id: number;
  code: string;
  rootId: number;
  name: string;
  order: number;
  userId: string;
  folderPath: string;
};

export type Node = {
  id: number;
  createTime: null;
  updateTime: null;
  app: number;
  userId: string;
  type: null;
  bookId: number;
  parentId: number | null;
  children: number[];
  title: string;
  name: string;
  text: string | null;
  data: null;
  /** Relative path under book root for .md files only (e.g. "Welcome.md") */
  relPath: string | null;
};

function isIgnoredName(name: string): boolean {
  if (IGNORED_DIR_NAMES.has(name)) return true;
  if (name.startsWith(".")) return true;
  return false;
}

function displayName(entryName: string, isFile: boolean): string {
  if (isFile && entryName.toLowerCase().endsWith(".md")) {
    return entryName.slice(0, -3);
  }
  return entryName;
}

/** Ensure resolved path stays under book root. */
export function assertUnderRoot(bookRoot: string, candidate: string): string {
  const root = resolve(bookRoot);
  const resolved = resolve(candidate);
  if (resolved !== root && !resolved.startsWith(root + sep)) {
    throw new Error(`Path escapes book folder: ${candidate}`);
  }
  return resolved;
}

type DirEntry = { name: string; path: string; isDirectory: boolean; isFile: boolean };

async function listEntries(dir: string): Promise<DirEntry[]> {
  const names = await readdir(dir);
  const entries: DirEntry[] = [];
  for (const name of names) {
    if (isIgnoredName(name)) continue;
    const path = join(dir, name);
    const s = await stat(path);
    if (s.isDirectory()) {
      entries.push({ name, path, isDirectory: true, isFile: false });
    } else if (s.isFile() && name.toLowerCase().endsWith(".md")) {
      entries.push({ name, path, isDirectory: false, isFile: true });
    }
  }
  entries.sort((a, b) => a.name.localeCompare(b.name));
  return entries;
}

/** Walk a registered book folder into goapi-shaped nodes. */
export async function scanBookFolder(record: BookRecord): Promise<{ book: Book; nodes: Node[] }> {
  const bookDir = resolve(record.path);
  const s = await stat(bookDir).catch(() => null);
  if (!s || !s.isDirectory()) {
    throw Object.assign(new Error(`Book folder not found: ${bookDir}`), { status: 404 });
  }

  const bookId = record.id;
  const nodes: Node[] = [];
  const idPrefix = bookDir; // absolute path as stable id namespace

  async function visit(
    absPath: string,
    parentId: number | null,
    isRoot: boolean,
  ): Promise<number> {
    assertUnderRoot(bookDir, absPath);
    const st = await stat(absPath);
    const isDir = st.isDirectory();
    const entryName = basename(absPath);
    const name = isRoot ? record.name : displayName(entryName, !isDir);
    const rel = isRoot ? "." : relative(bookDir, absPath).split(sep).join("/");
    const id = pathToId(`${idPrefix}:${rel}`);

    let text: string | null = null;
    const children: number[] = [];
    const relPath: string | null = isDir ? null : rel;

    if (isDir) {
      const entries = await listEntries(absPath);
      for (const entry of entries) {
        const childId = await visit(entry.path, id, false);
        children.push(childId);
      }
    } else {
      text = await readFile(absPath, "utf8");
    }

    nodes.push({
      id,
      createTime: null,
      updateTime: null,
      app: 1,
      userId: "local",
      type: null,
      bookId,
      parentId,
      children,
      title: name,
      name,
      text,
      data: null,
      relPath,
    });

    return id;
  }

  await visit(bookDir, null, true);
  const rootNode = nodes.find((n) => n.parentId === null);
  if (!rootNode) {
    throw new Error(`Missing root node for book ${record.name}`);
  }

  // Prefer display name on root
  rootNode.name = record.name;
  rootNode.title = record.name;

  const book: Book = {
    id: bookId,
    code: record.name,
    rootId: rootNode.id,
    name: record.name,
    order: record.order,
    userId: "local",
    folderPath: bookDir,
  };

  return { book, nodes };
}

export async function listBooksFromRegistry(records: BookRecord[]): Promise<Book[]> {
  const books: Book[] = [];
  const sorted = [...records].sort((a, b) => a.order - b.order);
  for (const record of sorted) {
    try {
      const { book } = await scanBookFolder(record);
      books.push(book);
    } catch (err) {
      // Still list the book even if folder temporarily missing — synthetic root
      const rootId = pathToId(`${resolve(record.path)}:.`);
      books.push({
        id: record.id,
        code: record.name,
        rootId,
        name: record.name,
        order: record.order,
        userId: "local",
        folderPath: resolve(record.path),
      });
      console.warn(`Book folder unavailable: ${record.path}`, err);
    }
  }
  return books;
}

/** Write markdown body for a file node; returns the updated node. */
export async function writeNodeText(
  record: BookRecord,
  nodeId: number,
  text: string,
): Promise<Node> {
  const { nodes } = await scanBookFolder(record);
  const node = nodes.find((n) => n.id === nodeId);
  if (!node) {
    throw Object.assign(new Error("Node not found"), { status: 404 });
  }
  if (!node.relPath) {
    throw Object.assign(new Error("Cannot write: node is not a markdown file"), { status: 400 });
  }

  const bookDir = resolve(record.path);
  const abs = assertUnderRoot(bookDir, join(bookDir, node.relPath));
  await writeFile(abs, text, "utf8");

  return { ...node, text };
}
