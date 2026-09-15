import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
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
  it('merges carousel edits made while a refresh is pending', async () => {
    let pagesGets = 0;
    let resolveRefresh!: (value: ReturnType<typeof response>) => void;
    const refresh = new Promise<ReturnType<typeof response>>((resolve) => { resolveRefresh = resolve; });
    const extra = { ...splash, id: 'old', name: 'Old', revision: 1 };
    const added = { ...splash, id: 'added', name: 'Added', revision: 5 };
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      if (path === '/api/pages' && ++pagesGets > 1) return refresh;
      if (path === '/api/pages/home' && options.method === 'PUT') return response({ ...home, revision: 4 });
      if (path === '/api/pages') return response({ schema_version: 1, revision: 7, carousel: ['home', 'splash', 'old'], pages: [summary(home), summary(splash), summary(extra)] });
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Save page' }));
    await waitFor(() => expect(pagesGets).toBe(2));
    await fireEvent.click(screen.getByRole('button', { name: 'Move Home down' }));
    await fireEvent.click(screen.getAllByRole('checkbox', { name: 'Enabled' })[1]);
    await fireEvent.change(screen.getByRole('spinbutton', { name: 'Duration Home' }), { target: { value: '15' } });
    resolveRefresh(response({ schema_version: 1, revision: 8, carousel: ['splash', 'home', 'added'], pages: [summary({ ...home, revision: 6 }), summary({ ...splash, revision: 4 }), summary(added)] }));
    await waitFor(() => expect(screen.getByRole('button', { name: /Added Revision 5/ })).toBeTruthy());
    expect(screen.queryByRole('button', { name: /Old Revision 1/ })).toBeNull();
    expect((screen.getByRole('spinbutton', { name: 'Duration Home' }) as HTMLInputElement).value).toBe('15');
    expect((screen.getAllByRole('checkbox', { name: 'Enabled' })[1] as HTMLInputElement).checked).toBe(false);
    expect(screen.getByRole('button', { name: 'Added Revision 5' })).toBeTruthy();
    expect(screen.getByText('Home', { selector: 'h2' })).toBeTruthy();
  });

  it('keeps a newer user selection during create', async () => {
    let resolveCreate!: (value: ReturnType<typeof response>) => void;
    const create = new Promise<ReturnType<typeof response>>((resolve) => { resolveCreate = resolve; });
    const created = { ...home, id: 'new', name: 'New page', revision: 1 };
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      if (path === '/api/pages' && options.method === 'POST') return create;
      if (path === '/api/pages/new') return response(created);
      if (path === '/api/pages') return response({ ...initial(path), pages: [summary(home), summary(splash), summary(created)] });
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Create page' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Splash Revision 2' }));
    resolveCreate(response(created));
    await waitFor(() => expect(screen.getByText('Splash', { selector: 'h2' })).toBeTruthy());
    expect(fetchMock).not.toHaveBeenCalledWith('/api/pages/new', expect.anything());
  });

  it('keeps a newer user selection while a page save is pending', async () => {
    let resolveSave!: (value: ReturnType<typeof response>) => void;
    const save = new Promise<ReturnType<typeof response>>((resolve) => { resolveSave = resolve; });
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      if (path === '/api/pages/home' && options.method === 'PUT') return save;
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Save page' }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith('/api/pages/home', expect.objectContaining({ method: 'PUT' })));
    await fireEvent.click(screen.getByRole('button', { name: 'Splash Revision 2' }));
    await waitFor(() => expect(screen.getByText('Splash', { selector: 'h2' })).toBeTruthy());
    resolveSave(response({ ...home, revision: 4 }));
    await screen.findByText('Page saved');
    expect(screen.getByText('Splash', { selector: 'h2' })).toBeTruthy();
  });

  it('keeps a newer user selection while deleting the current page', async () => {
    let deleted = false;
    let resolveDelete!: (value: ReturnType<typeof response>) => void;
    let resolveSplash!: (value: ReturnType<typeof response>) => void;
    const deletion = new Promise<ReturnType<typeof response>>((resolve) => { resolveDelete = resolve; });
    const splashLoad = new Promise<ReturnType<typeof response>>((resolve) => { resolveSplash = resolve; });
    const storage = { ...home, id: 'storage', name: 'Storage', revision: 1, template_id: 'factory.storage.v1' };
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      if (path === '/api/pages/home' && options.method === 'DELETE') return deletion;
      if (path === '/api/pages') {
        const listed = deleted ? [summary(storage), summary(splash)] : [summary(home), summary(storage), summary(splash)];
        return response({ schema_version: 1, revision: deleted ? 8 : 7, carousel: listed.map(({ id }) => id), pages: listed });
      }
      if (path === '/api/pages/splash') return splashLoad;
      if (path === '/api/pages/storage') return response(storage);
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver); vi.stubGlobal('confirm', vi.fn(() => true));
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Delete Home' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Splash Revision 2' }));
    deleted = true;
    resolveDelete(response({}));
    await screen.findByText('Page deleted');
    resolveSplash(response(splash));
    await waitFor(() => expect(screen.getByText('Splash', { selector: 'h2' })).toBeTruthy());
    expect(fetchMock).not.toHaveBeenCalledWith('/api/pages/storage', expect.anything());
  });

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

  it('keeps a newer user selection while Orbit creation is pending', async () => {
    let resolveOrbit!: (value: ReturnType<typeof response>) => void;
    const orbit = new Promise<ReturnType<typeof response>>((resolve) => { resolveOrbit = resolve; });
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      if (path === '/api/media/presets/orbit') return orbit;
      if (path === '/api/carousel') return response({ ok: true, revision: 9 });
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: /Media$/ }));
    await fireEvent.change(screen.getByRole('combobox', { name: 'Source' }), { target: { value: 'logo' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Créer / mettre à jour Orbit' }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith('/api/media/presets/orbit', expect.objectContaining({ method: 'POST' })));
    await fireEvent.click(screen.getByRole('button', { name: /Pages$/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Home Revision 3' }));
    resolveOrbit(response({ id: 'orbit', name: 'Orbit', source_asset_id: 'logo', settings: { fps: 24, speed_seconds: 4 } }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith('/api/apply', expect.objectContaining({ method: 'POST' })));
    expect(screen.getByText('Home', { selector: 'h2' })).toBeTruthy();
    expect(fetchMock).not.toHaveBeenCalledWith('/api/pages/splash', expect.anything());
  });

  it('previews the returned Splash page after a newer selection', async () => {
    let resolveSplash!: (value: ReturnType<typeof response>) => void;
    const splashLoad = new Promise<ReturnType<typeof response>>((resolve) => { resolveSplash = resolve; });
    const previewPages: typeof home[] = [];
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      if (path === '/api/media') return response({ assets: [{ id: 'logo', name: 'Logo' }], presets: [{ id: 'orbit', name: 'Orbit', source_asset_id: 'logo', settings: { fps: 24, speed_seconds: 4 } }] });
      if (path === '/api/pages/splash') return splashLoad;
      if (path === '/api/preview') {
        previewPages.push(JSON.parse(String(options.body)).page);
        return { ok: true, status: 200, blob: async () => new Blob() };
      }
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: /Media$/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Aperçu Orbit' }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith('/api/pages/splash', expect.anything()));
    await fireEvent.click(screen.getByRole('button', { name: /Pages$/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Home Revision 3' }));
    resolveSplash(response(splash));
    await waitFor(() => expect(previewPages).toHaveLength(1));
    expect(screen.getByText('Home', { selector: 'h2' })).toBeTruthy();
    expect(previewPages).toEqual([splash]);
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
  it('uses a canvas-first studio with friendly metrics and no permanent inspector', async () => {
    const boundHome = { ...home, layers: [{ id: 'cpu', type: 'gauge', binding: 'aooscope_pve_cpu_pct', x: 20, y: 20, width: 180, height: 120, z: 1 }] };
    const fetchMock = vi.fn(async (path: string) => {
      if (path === '/api/pages/home') return response(boundHome);
      if (path === '/api/metrics') return response({ metrics: [{ id: 'aooscope_pve_cpu_pct', label: 'Utilisation CPU', provider_name: 'Proxmox', category: 'CPU', value: 42, demo_value: 50, unit: '%', recommended_widgets: ['gauge', 'value', 'bar'] }] });
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    expect(screen.getByTestId('page-strip')).toBeTruthy();
    expect(screen.getByRole('button', { name: '+ Metric' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Templates' })).toBeTruthy();
    expect(screen.getByText('Utilisation CPU')).toBeTruthy();
    expect(screen.queryByText('aooscope_pve_cpu_pct')).toBeNull();
    expect(screen.queryByText('Inspecteur')).toBeNull();
  });

  it('applies contextual representation changes to the selected layer', async () => {
    const boundHome = { ...home, layers: [{ id: 'cpu', type: 'gauge', binding: 'aooscope_pve_cpu_pct', x: 20, y: 20, width: 180, height: 120, z: 1 }] };
    const fetchMock = vi.fn(async (path: string) => {
      if (path === '/api/pages/home') return response(boundHome);
      if (path === '/api/metrics') return response({ metrics: [{ id: 'aooscope_pve_cpu_pct', label: 'Utilisation CPU', provider_name: 'Proxmox', category: 'CPU', value: 42, demo_value: 50, unit: '%', recommended_widgets: ['gauge', 'value', 'bar'] }] });
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    const layer = await screen.findByRole('button', { name: 'Utilisation CPU 42%' });
    await fireEvent.click(layer);
    await fireEvent.click(screen.getByRole('button', { name: 'Bar' }));
    expect(layer.classList.contains('bar')).toBe(true);
  });

  it('inserts a metric with friendly canvas content and hides its binding', async () => {
    const metric = { id: 'aooscope_pve_cpu_pct', label: 'Utilisation CPU', provider_name: 'Proxmox', category: 'CPU', value: 42, demo_value: 50, unit: '%', recommended_widgets: ['gauge', 'value', 'bar'] };
    const fetchMock = vi.fn(async (path: string) => path === '/api/metrics' ? response({ metrics: [metric] }) : response(initial(path)));
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: '+ Metric' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Add Utilisation CPU' }));
    const canvas = within(screen.getByTestId('logical-canvas'));
    expect(canvas.getByText('Utilisation CPU')).toBeTruthy();
    expect(canvas.getByText('42%')).toBeTruthy();
    expect(screen.queryByText(metric.id)).toBeNull();
  });

  it('adds bound text metrics without replacing their value or changing static text defaults', async () => {
    const metric = { id: 'aooscope_pve_disks_0_name', label: 'Disque 1 · nom', provider_name: 'Proxmox', category: 'Stockage', value: 'NVMe', demo_value: 'Disk 1', unit: '', recommended_widgets: ['text'] };
    const fetchMock = vi.fn(async (path: string) => path === '/api/metrics' ? response({ metrics: [metric] }) : response(initial(path)));
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByText('More widgets'));
    await fireEvent.click(screen.getByRole('button', { name: 'Ajouter Texte' }));
    const canvas = within(screen.getByTestId('logical-canvas'));
    expect(canvas.getByText('Text')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: '+ Metric' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Add Disque 1 · nom' }));
    expect(canvas.getByText('Disque 1 · nom')).toBeTruthy();
    expect(canvas.getAllByText('Text')).toHaveLength(1);
  });

  it('creates a template through template_id and selects the returned page', async () => {
    const created = { ...home, id: 'semi', name: 'Semi rings', revision: 1, template_id: 'factory.semi-rings.v1' };
    let createdVisible = false;
    const fetchMock = vi.fn(async (path: string, options: RequestInit = {}) => {
      if (path === '/api/pages' && options.method === 'POST') { createdVisible = true; return response(created); }
      if (path === '/api/pages' && createdVisible) return response({ schema_version: 1, revision: 8, carousel: ['home', 'splash', 'semi'], pages: [summary(home), summary(splash), summary(created)] });
      if (path === '/api/pages/semi') return response(created);
      return response(initial(path));
    });
    vi.stubGlobal('fetch', fetchMock); vi.stubGlobal('EventSource', FakeEventSource); vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    render(PageRoute);
    await screen.findByText('Home', { selector: 'h2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Templates' }));
    await fireEvent.click(screen.getByRole('button', { name: /Semi rings/ }));
    await screen.findByText('Semi rings', { selector: 'h2' });
    const request = fetchMock.mock.calls.find(([path, options]) => path === '/api/pages' && options?.method === 'POST');
    expect(JSON.parse(String(request?.[1]?.body))).toEqual({ template_id: 'factory.semi-rings.v1' });
  });

});
