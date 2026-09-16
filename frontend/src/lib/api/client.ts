import type { paths } from './schema';


async function responseError(path: string, response: Response): Promise<Error> {
  let detail = '';
  try {
    const payload = await response.clone().json() as { error?: unknown };
    if (typeof payload?.error === 'string') detail = payload.error.trim();
  } catch {
    try { detail = (await response.clone().text()).trim(); } catch { detail = ''; }
  }
  return new Error(detail ? `${path}: ${detail} (HTTP ${response.status})` : `${path}: HTTP ${response.status}`);
}

type OkJson<P extends keyof paths> = paths[P] extends {
  get: { responses: { 200: { content: { 'application/json': infer T } } } }
} ? T : never;

export async function getJson<P extends keyof paths>(path: P): Promise<OkJson<P>> {
  const response = await fetch(path as string, {
    headers: { Accept: 'application/json' }
  });
  if (!response.ok) throw await responseError(String(path), response);
  return await response.json() as OkJson<P>;
}

export async function requestJson<T>(path: string, method = 'GET', body?: unknown): Promise<T> {
  const response = await fetch(path, { method, headers: { Accept: 'application/json', ...(body ? { 'Content-Type': 'application/json' } : {}) }, body: body === undefined ? undefined : JSON.stringify(body) });
  if (!response.ok) throw await responseError(path, response);
  if (response.status === 204) return undefined as T;
  return await response.json() as T;
}

export async function requestBlob(path: string, method = 'GET', body?: unknown): Promise<Blob> {
  const response = await fetch(path, { method, headers: { Accept: 'image/png', ...(body ? { 'Content-Type': 'application/json' } : {}) }, body: body === undefined ? undefined : JSON.stringify(body) });
  if (!response.ok) throw await responseError(path, response);
  return response.blob();
}
