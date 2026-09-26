import { readdir, readFile, stat } from "fs/promises";
import { basename, join, relative, resolve, sep } from "path";
import { bookIdFor, pathToId } from "./ids";

const IGNORED_DIR_NAMES = new Set([".obsidian", ".git", "node_modules"]);

export type Book = {
  id: number;
  code: string;
  rootId: number;
  name: string;
  order: number;
  userId: string;
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
};

export type VaultSnapshot = {
  books: Book[];
  nodesByBookId: Map<number, Node[]>;
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

/** Ensure resolved path stays under vault root. */
export function assertUnderVault(vaultPath: string, candidate: string): string {
  const root = resolve(vaultPath);
  const resolved = resolve(candidate);
  if (resolved !== root && !resolved.startsWith(root + sep)) {
    throw new Error(`Path escapes vault: ${candidate}`);
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

async function walkBook(
  vaultPath: string,
  bookDir: string,
  bookId: number,
  rootRel: string,
): Promise<Node[]> {
  const nodes: Node[] = [];

  async function visit(
    absPath: string,
    relPath: string,
    parentId: number | null,
    isRoot: boolean,
  ): Promise<number> {
    assertUnderVault(vaultPath, absPath);
    const s = await stat(absPath);
    const isDir = s.isDirectory();
    const entryName = basename(absPath);
    const name = isRoot ? basename(bookDir) : displayName(entryName, !isDir);
    const id = pathToId(relPath);

    let text: string | null = null;
    const children: number[] = [];

    if (isDir) {
      const entries = await listEntries(absPath);
      for (const entry of entries) {
        const childRel = relative(vaultPath, entry.path).split(sep).join("/");
        const childId = await visit(entry.path, childRel, id, false);
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
    });

    return id;
  }

  await visit(bookDir, rootRel, null, true);
  return nodes;
}

export async function scanVault(vaultPath: string): Promise<VaultSnapshot> {
  const root = resolve(vaultPath);
  const rootStat = await stat(root).catch(() => null);
  if (!rootStat || !rootStat.isDirectory()) {
    throw new Error(`VAULT_PATH is not a directory: ${root}`);
  }

  const top = await readdir(root);
  const topDirs: string[] = [];
  const topMd: string[] = [];

  for (const name of top) {
    if (isIgnoredName(name)) continue;
    const path = join(root, name);
    const s = await stat(path);
    if (s.isDirectory()) topDirs.push(name);
    else if (s.isFile() && name.toLowerCase().endsWith(".md")) topMd.push(name);
  }

  topDirs.sort((a, b) => a.localeCompare(b));

  const books: Book[] = [];
  const nodesByBookId = new Map<number, Node[]>();

  if (topDirs.length === 0) {
    // Synthetic book: entire vault as one book
    const bookName = basename(root);
    const bookId = bookIdFor(bookName);
    const rootRel = ".";
    const rootId = pathToId(rootRel);

    // Build a root node for the vault, with top-level md (and any nested dirs we missed — none)
    const nodes: Node[] = [];
    const children: number[] = [];

    for (const name of topMd.sort((a, b) => a.localeCompare(b))) {
      const abs = join(root, name);
      const rel = name;
      const id = pathToId(rel);
      const display = displayName(name, true);
      const text = await readFile(abs, "utf8");
      nodes.push({
        id,
        createTime: null,
        updateTime: null,
        app: 1,
        userId: "local",
        type: null,
        bookId,
        parentId: rootId,
        children: [],
        title: display,
        name: display,
        text,
        data: null,
      });
      children.push(id);
    }

    nodes.unshift({
      id: rootId,
      createTime: null,
      updateTime: null,
      app: 1,
      userId: "local",
      type: null,
      bookId,
      parentId: null,
      children,
      title: bookName,
      name: bookName,
      text: null,
      data: null,
    });

    books.push({
      id: bookId,
      code: bookName,
      rootId,
      name: bookName,
      order: 0,
      userId: "local",
    });
    nodesByBookId.set(bookId, nodes);
    return { books, nodesByBookId };
  }

  for (let i = 0; i < topDirs.length; i++) {
    const folderName = topDirs[i]!;
    const bookDir = join(root, folderName);
    const bookId = bookIdFor(folderName);
    const rootRel = folderName;
    const nodes = await walkBook(root, bookDir, bookId, rootRel);
    const rootNode = nodes.find((n) => n.parentId === null);
    if (!rootNode) {
      throw new Error(`Missing root node for book ${folderName}`);
    }

    books.push({
      id: bookId,
      code: folderName,
      rootId: rootNode.id,
      name: folderName,
      order: i,
      userId: "local",
    });
    nodesByBookId.set(bookId, nodes);
  }

  return { books, nodesByBookId };
}
