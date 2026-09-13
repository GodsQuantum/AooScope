import { afterEach, describe, expect, it, vi } from 'vitest';
import { requestBlob } from './client';

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
