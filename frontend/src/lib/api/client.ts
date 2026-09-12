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
