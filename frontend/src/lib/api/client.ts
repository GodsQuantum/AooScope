import type { paths } from './schema';

type OkJson<P extends keyof paths> = paths[P] extends {
  get: { responses: { 200: { content: { 'application/json': infer T } } } }
} ? T : never;

export async function getJson<P extends keyof paths>(path: P): Promise<OkJson<P>> {
  const response = await fetch(path as string, {
    headers: { Accept: 'application/json' }
  });
  if (!response.ok) {
    throw new Error(`${String(path)}: HTTP ${response.status}`);
  }
  return await response.json() as OkJson<P>;
}

export async function requestJson<T>(path: string, method = 'GET', body?: unknown): Promise<T> {
  const response = await fetch(path, { method, headers: { Accept: 'application/json', ...(body ? { 'Content-Type': 'application/json' } : {}) }, body: body === undefined ? undefined : JSON.stringify(body) });
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return await response.json() as T;
}

export async function requestBlob(path: string, method = 'GET', body?: unknown): Promise<Blob> {
  const response = await fetch(path, { method, headers: { Accept: 'image/png', ...(body ? { 'Content-Type': 'application/json' } : {}) }, body: body === undefined ? undefined : JSON.stringify(body) });
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return response.blob();
}
