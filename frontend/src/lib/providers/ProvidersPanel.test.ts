import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ProvidersPanel from './ProvidersPanel.svelte';

const catalog = [{ id: 'proxmox', name: 'Proxmox', icon: 'server', categories: ['compute'], credential_fields: ['api_token'] }];
const settings = { proxmox: { enabled: true, url: 'https://example.test', verify_tls: true, node: 'node-a', secret_set: true } };
const statuses = [{ id: 'proxmox', configured: true, enabled: true, online: true, last_success: 1_789_344_000, error: null }];

afterEach(() => vi.unstubAllGlobals());

describe('ProvidersPanel', () => {
  it('shows secret-set and live collection state without reading a secret back', () => {
    render(ProvidersPanel, { props: { catalog, settings, statuses } });
    expect(screen.getByText('Credential saved')).toBeTruthy();
    expect(screen.getByText('Online')).toBeTruthy();
    expect(screen.getByText(/Last success/)).toBeTruthy();
    expect((screen.getByLabelText('API token') as HTMLInputElement).value).toBe('');
    expect((screen.getByLabelText('Node') as HTMLInputElement).value).toBe('node-a');
  });

  it('wires edited provider fields to save and test routes', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: async () => ({ display: {}, providers: settings }) })
      .mockResolvedValueOnce({ ok: true, json: async () => ({ display: {}, providers: settings }) })
      .mockResolvedValueOnce({ ok: true, json: async () => statuses[0] });
    vi.stubGlobal('fetch', fetchMock);
    render(ProvidersPanel, { props: { catalog, settings, statuses } });
    await fireEvent.input(screen.getByLabelText('API token'), { target: { value: 'new-secret' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save providers' }));
    expect(fetchMock).toHaveBeenNthCalledWith(1, '/api/settings', expect.objectContaining({
      method: 'PUT', body: expect.stringContaining('new-secret')
    }));
    await fireEvent.click(screen.getByRole('button', { name: 'Test Proxmox' }));
    await waitFor(() => expect(fetchMock).toHaveBeenNthCalledWith(3, '/api/providers/proxmox/test', expect.objectContaining({ method: 'POST' })));
  });

  it('does not test stale settings when save fails', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: false, status: 500 });
    vi.stubGlobal('fetch', fetchMock);
    render(ProvidersPanel, { props: { catalog, settings, statuses } });
    await fireEvent.click(screen.getByRole('button', { name: 'Test Proxmox' }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(fetchMock).toHaveBeenCalledWith('/api/settings', expect.objectContaining({ method: 'PUT' }));
  });
});
