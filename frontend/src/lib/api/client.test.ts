import { afterEach, describe, expect, it, vi } from 'vitest';
import { requestBlob, requestJson } from './client';

afterEach(() => vi.unstubAllGlobals());

describe('requestBlob', () => {
  it('requests binary content and returns the response blob', async () => {
    const blob = new Blob(['png'], { type: 'image/png' });
    const fetchMock = vi.fn().mockResolvedValue(new Response(blob, { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);

    await expect(requestBlob('/api/preview', 'POST', { page: {} })).resolves.toEqual(blob);
    expect(fetchMock).toHaveBeenCalledWith('/api/preview', {
      method: 'POST',
      headers: { Accept: 'image/png', 'Content-Type': 'application/json' },
      body: JSON.stringify({ page: {} })
    });
  });
});

describe('requestJson', () => {
  it('accepts successful empty responses', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 204 })));
    await expect(requestJson<void>('/api/media/asset', 'DELETE')).resolves.toBeUndefined();
  });
});
