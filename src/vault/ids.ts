/** Deterministic positive 32-bit id from a vault-relative path key. */
export function pathToId(key: string): number {
  let hash = 2166136261;
  for (let i = 0; i < key.length; i++) {
    hash ^= key.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  // Force unsigned 32-bit, then keep positive (avoid 0)
  const unsigned = hash >>> 0;
  return unsigned === 0 ? 1 : unsigned;
}

export function bookIdKey(folderName: string): string {
  return `book:${folderName}`;
}

export function bookIdFor(folderName: string): number {
  return pathToId(bookIdKey(folderName));
}
