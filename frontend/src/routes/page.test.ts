import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import PageRoute from './+page.svelte';

class FakeEventSource {
  addEventListener() {}
  close() {}
}
class FakeResizeObserver {
  observe() {}
  disconnect() {}
}

const home = { id: 'home', name: 'Home', enabled: true, duration: 8, revision: 3, template_id: 'factory.home.v1', background: { color: '#071019' }, layers: [] };
const splash = { id: 'splash', name: 'Splash', enabled: true, duration: 8, revision: 2, template_id: 'factory.splash.v1', background: { color: '#071019' }, layers: [] };
const summary = ({ background: _background, layers: _layers, ...page }: typeof home) => page;

function response(json: unknown, ok = true) {
  return { ok, status: ok ? 200 : 500, json: async () => json };
}

function initial(path: string) {
  if (path === '/api/pages') return { schema_version: 1, revision: 7, carousel: ['home', 'splash'], pages: [summary(home), summary(splash)] };
  if (path === '/api/pages/home') return home;
  if (path === '/api/pages/splash') return splash;
  if (path === '/api/metrics') return { metrics: [] };
  if (path === '/api/providers/catalog') return { providers: [] };
  if (path === '/api/media') return { assets: [{ id: 'logo', name: 'Logo' }], presets: [] };
  if (path === '/api/display/capabilities') return { width: 960, height: 376, native_brightness: false, power_control: true, power_on: true };
  if (path === '/api/settings') return { display: {}, providers: {} };
  if (path === '/api/providers/status') return [];
  if (path === '/api/status') return { version: 'test', brightness: 100, native_brightness: false, device_present: false, updated_unix: null };
  return {};
}

afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe('admin page persistence', () => {
  it('saves the page then carousel at the refreshed revision before applying', async () => {
    const writes: { path: string; body: any }[] = [];
    let pagesGets = 0;
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      const method = options.method ?? 'GET';
      const body = options.body ? JSON.parse(String(options.body)) : undefined;
      if (method !== 'GET') writes.push({ path, body });
      if (path === '/api/pages' && ++pagesGets > 1) return response({ ...initial(path), revision: 8 });
      if (path === '/api/pages/home' && method === 'PUT') return response({ ...home, ...body, revision: 4 });
      if (path === '/api/carousel' && method === 'PUT') return response({ ok: true, revision: 9 });
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.change(screen.getByRole('spinbutton', { name: 'Duration Home' }), { target: { value: '15' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Ajouter Texte' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Apply to LCD' }));
    await waitFor(() => expect(writes.map(({ path }) => path)).toEqual(['/api/pages/home', '/api/carousel', '/api/apply']));
    expect(writes[0].body.layers).toHaveLength(1);
    expect(writes[0].body).not.toHaveProperty('enabled');
    expect(writes[0].body).not.toHaveProperty('duration');
    expect(writes[1].body.revision).toBe(8);
    expect(writes[1].body.items[0]).toMatchObject({ id: 'home', duration: 15 });
  });

  it('does not apply when saving the page fails', async () => {
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => path === '/api/pages/home' && options.method === 'PUT' ? response({}, false) : response(initial(path)));
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Apply to LCD' }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith('/api/pages/home', expect.objectContaining({ method: 'PUT' })));
    expect(fetchMock).not.toHaveBeenCalledWith('/api/carousel', expect.anything());
    expect(fetchMock).not.toHaveBeenCalledWith('/api/apply', expect.anything());
  });

  it('preserves carousel drafts when a page save refreshes summaries', async () => {
    let pagesGets = 0;
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      const body = options.body ? JSON.parse(String(options.body)) : undefined;
      if (path === '/api/pages' && ++pagesGets > 1) return response({ ...initial(path), revision: 8 });
      if (path === '/api/pages/home' && options.method === 'PUT') return response({ ...home, ...body, revision: 4 });
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Move Home down' }));
    await fireEvent.click(screen.getAllByRole('checkbox', { name: 'Enabled' })[1]);
    await fireEvent.change(screen.getByRole('spinbutton', { name: 'Duration Home' }), { target: { value: '15' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save page' }));
    await waitFor(() => expect((screen.getByRole('spinbutton', { name: 'Duration Home' }) as HTMLInputElement).value).toBe('15'));
    expect((screen.getByRole('button', { name: 'Move Home down' }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getAllByRole('checkbox', { name: 'Enabled' })[1] as HTMLInputElement).checked).toBe(false);
    const pageWrite = fetchMock.mock.calls.find(([path, options]) => path === '/api/pages/home' && options?.method === 'PUT');
    const pageBody = JSON.parse(String(pageWrite?.[1]?.body));
    expect(pageBody).not.toHaveProperty('enabled');
    expect(pageBody).not.toHaveProperty('duration');
  });

  it('refreshes the Orbit revision, preserves carousel drafts, then applies', async () => {
    const writes: { path: string; body: any }[] = [];
    let pagesGets = 0;
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      const method = options.method ?? 'GET'; const body = options.body ? JSON.parse(String(options.body)) : undefined;
      if (method !== 'GET') writes.push({ path, body });
      if (path === '/api/pages' && ++pagesGets > 1) return response({ ...initial(path), revision: 8, pages: [summary(home), { ...summary(splash), revision: 3 }] });
      if (path === '/api/media/presets/orbit') return response({ id: 'orbit', name: 'Orbit', source_asset_id: 'logo', settings: { fps: 24, speed_seconds: 4 } });
      if (path === '/api/pages/splash') return response({ ...splash, revision: 3 });
      if (path === '/api/carousel') return response({ ok: true, revision: 9 });
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.change(screen.getByRole('spinbutton', { name: 'Duration Home' }), { target: { value: '15' } });
    await fireEvent.click(screen.getByRole('button', { name: /Media$/ }));
    await fireEvent.change(screen.getByRole('combobox', { name: 'Source' }), { target: { value: 'logo' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Créer / mettre à jour Orbit' }));
    await waitFor(() => expect(writes.map(({ path }) => path)).toEqual(['/api/media/presets/orbit', '/api/carousel', '/api/apply']));
    expect(writes[1].body).toMatchObject({ revision: 8, items: [expect.objectContaining({ id: 'home', duration: 15 }), expect.anything()] });
  });

  it('confirms before restoring a factory page', async () => {
    const fetchMock = vi.fn(async (path: string) => response(initial(path)));
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver); vi.stubGlobal('confirm', vi.fn(() => false));
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Restore Home' }));
    expect(confirm).toHaveBeenCalled();
    expect(fetchMock).not.toHaveBeenCalledWith('/api/pages/home/restore', expect.anything());
  });

  it('preserves carousel drafts while integrating page CRUD responses', async () => {
    let listed = [summary(home), summary(splash)];
    const created = { ...home, id: 'new', name: 'New page', revision: 1 };
    const duplicated = { ...splash, id: 'copy', name: 'Splash copy', revision: 1 };
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      const method = options.method ?? 'GET';
      if (path === '/api/pages' && method === 'GET') return response({ schema_version: 1, revision: 8, carousel: listed.map((item) => item.id), pages: listed });
      if (path === '/api/pages/new') return response(created);
      if (path === '/api/pages/copy') return response(duplicated);
      if (path === '/api/pages/home/restore') return response({ ...home, revision: 9 });
      if (path === '/api/pages/home' && method === 'DELETE') { listed = [summary(splash), summary(created), summary(duplicated)]; return response({}); }
      if (path === '/api/pages' && method === 'POST') { listed = [...listed, summary(created)]; return response(created); }
      if (path === '/api/pages/splash/duplicate') { listed = [...listed, summary(duplicated)]; return response(duplicated); }
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver); vi.stubGlobal('confirm', vi.fn(() => true));
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Move Home down' }));
    await fireEvent.click(screen.getAllByRole('checkbox', { name: 'Enabled' })[1]);
    await fireEvent.change(screen.getByRole('spinbutton', { name: 'Duration Home' }), { target: { value: '15' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Create page' }));
    await waitFor(() => expect((screen.getByRole('spinbutton', { name: 'Duration Home' }) as HTMLInputElement).value).toBe('15'));
    await fireEvent.click(screen.getByRole('button', { name: 'Duplicate Splash' }));
    await waitFor(() => expect(screen.getByText('Splash copy')).toBeTruthy());
    await fireEvent.click(screen.getByRole('button', { name: 'Restore Home' }));
    await waitFor(() => expect((screen.getByRole('spinbutton', { name: 'Duration Home' }) as HTMLInputElement).value).toBe('15'));
    await fireEvent.click(screen.getByRole('button', { name: 'Delete Home' }));
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Delete Home' })).toBeNull());
    expect((screen.getByRole('spinbutton', { name: 'Duration Splash' }) as HTMLInputElement).value).toBe('8');
    expect(fetchMock).toHaveBeenCalledWith('/api/pages/home/restore', expect.objectContaining({ method: 'POST' }));
  });
});
