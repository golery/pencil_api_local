import { mkdir, readFile, writeFile } from "fs/promises";
import { dirname, resolve } from "path";
import { bookIdFor } from "./ids";

export type BookRecord = {
  id: number;
  name: string;
  path: string;
  order: number;
};

async function ensureFile(booksFile: string): Promise<void> {
  await mkdir(dirname(booksFile), { recursive: true });
  try {
    await readFile(booksFile, "utf8");
  } catch {
    await writeFile(booksFile, "[]\n", "utf8");
  }
}

export async function loadRegistry(booksFile: string): Promise<BookRecord[]> {
  await ensureFile(booksFile);
  const raw = await readFile(booksFile, "utf8");
  const parsed = JSON.parse(raw || "[]");
  if (!Array.isArray(parsed)) {
    throw new Error("books.json must be an array");
  }
  return parsed as BookRecord[];
}

export async function saveRegistry(booksFile: string, records: BookRecord[]): Promise<void> {
  await ensureFile(booksFile);
  await writeFile(booksFile, JSON.stringify(records, null, 2) + "\n", "utf8");
}

export async function addBook(
  booksFile: string,
  name: string,
  folderPath: string,
): Promise<BookRecord> {
  const abs = resolve(folderPath);
  const records = await loadRegistry(booksFile);
  if (records.some((r) => resolve(r.path) === abs)) {
    throw Object.assign(new Error("Folder is already linked"), { status: 409 });
  }
  const record: BookRecord = {
    id: bookIdFor(abs),
    name: name.trim(),
    path: abs,
    order: records.length === 0 ? 0 : Math.max(...records.map((r) => r.order)) + 1,
  };
  records.push(record);
  await saveRegistry(booksFile, records);
  return record;
}

export async function updateBookName(
  booksFile: string,
  bookId: number,
  name: string,
): Promise<BookRecord> {
  const records = await loadRegistry(booksFile);
  const record = records.find((r) => r.id === bookId);
  if (!record) {
    throw Object.assign(new Error("Book not found"), { status: 404 });
  }
  record.name = name.trim();
  await saveRegistry(booksFile, records);
  return record;
}

export async function removeBook(booksFile: string, bookId: number): Promise<void> {
  const records = await loadRegistry(booksFile);
  const next = records.filter((r) => r.id !== bookId);
  if (next.length === records.length) {
    throw Object.assign(new Error("Book not found"), { status: 404 });
  }
  await saveRegistry(booksFile, next);
}

export async function getBookRecord(
  booksFile: string,
  bookId: number,
): Promise<BookRecord | undefined> {
  const records = await loadRegistry(booksFile);
  return records.find((r) => r.id === bookId);
}
