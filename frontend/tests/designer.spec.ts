import { expect, test, type Page } from '@playwright/test';

const summaries = [
  { id: 'page-home', name: 'Home', enabled: true, duration: 8, revision: 3, template_id: 'factory.home.v1' },
  { id: 'page-storage', name: 'Storage', enabled: true, duration: 10, revision: 2, template_id: 'factory.storage.v1' },
  { id: 'page-compute', name: 'Compute', enabled: true, duration: 8, revision: 1, template_id: 'factory.compute.v1' },
  { id: 'page-media', name: 'Media', enabled: true, duration: 8, revision: 1, template_id: 'factory.media.v1' }
];
const output = '../.superpowers/sdd/visual-studio-redesign/task-4-screenshots';
const home = { ...summaries[0], background: { color: '#071019' }, layers: [
  { id: 'brand', type: 'text', x: 28, y: 24, width: 260, height: 32, z: 8, text: 'AOOSCOPE', color: '#eaf7ff' },
  { id: 'accent', type: 'badge', x: 28, y: 70, width: 120, height: 10, z: 8, background_color: '#35d9ff', radius: 5 },
  { id: 'cpu', type: 'gauge', x: 45, y: 100, width: 210, height: 210, z: 1, color: '#35d9ff', binding: 'CPU 42%' },
  { id: 'ram', type: 'gauge', x: 375, y: 100, width: 210, height: 210, z: 1, color: '#58e5a4', binding: 'RAM 68%' },
  { id: 'temp', type: 'gauge', x: 705, y: 100, width: 210, height: 210, z: 1, color: '#ffc35d', binding: 'CPU TEMP 54 C' }
] };
const providers = [
  ['local', 'Local hardware', []], ['proxmox', 'Proxmox', ['api_token']], ['beszel', 'Beszel', ['email', 'password']], ['jellyfin', 'Jellyfin', ['api_key']], ['silo', 'Silo', ['api_key']],
  ['radarr', 'Radarr', ['api_key']], ['sonarr', 'Sonarr', ['api_key']], ['qbittorrent', 'qBittorrent', ['username', 'password']], ['immich', 'Immich', ['api_key']], ['ollama', 'Ollama', []]
].map(([id, name, credential_fields]) => ({ id, name, icon: 'service', categories: id === 'local' ? ['hardware'] : ['telemetry'], credential_fields } as { id: string; name: string; icon: string; categories: string[]; credential_fields: string[] }));

async function mockApi(page: Page) {
  let listed = [...summaries];
  const semiRings = { id: 'page-semi', name: 'Semi rings', enabled: true, duration: 8, revision: 1, template_id: 'factory.semi-rings.v1', background: { color: '#071019' }, layers: [] };
  await page.route('**/api/**', async (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    let json: unknown = {};
    if (path === '/api/pages' && method === 'POST') {
      const body = route.request().postDataJSON();
      if (body.template_id === 'factory.semi-rings.v1') { listed = [...listed, semiRings]; json = semiRings; }
    } else json = path === '/api/pages' ? { schema_version: 1, revision: listed.length === summaries.length ? 7 : 8, carousel: listed.map((item) => item.id), pages: listed }
      : path === '/api/pages/page-home' ? home
      : path === '/api/pages/page-semi' ? semiRings
      : path === '/api/metrics' ? { metrics: [{ id: 'aooscope_pve_cpu_pct', label: 'CPU load', provider_id: 'proxmox', provider_name: 'Proxmox', category: 'Compute', value_type: 'number', unit: '%', value: 42, demo_value: 35, online: true, recommended_widgets: ['gauge', 'value'] }] }
      : path === '/api/providers/catalog' ? { providers }
      : path === '/api/providers/status' ? providers.map(({ id }) => ({ id, configured: id === 'local' || id === 'proxmox', enabled: id === 'local' || id === 'proxmox', online: id === 'local' || id === 'proxmox', last_success: id === 'local' || id === 'proxmox' ? 1_789_344_000 : null, error: null }))
      : path === '/api/media' ? { assets: [{ id: 'logo', name: 'AooScope mark', kind: 'image', format: 'PNG', revision: 1, width: 960, height: 376 }], presets: [{ id: 'orbit', name: 'AooScope Orbit', source_asset_id: 'logo', settings: { fps: 24, speed_seconds: 4 } }] }
      : path === '/api/display/capabilities' ? { width: 960, height: 376, native_brightness: false, power_control: true, power_on: true }
      : path === '/api/settings' ? { display: { brand: 'AOOSCOPE', brightness: 78, schedule_enabled: true, timezone: 'UTC', switch_seconds: 8, schedule: [{ start: '22:00', end: '08:00', brightness: 60 }] }, providers: { proxmox: { enabled: true, url: 'https://cluster.example.test', verify_tls: true, node: 'node-a', secret_set: true } } }
      : path === '/api/status' ? { version: '0.3.0-dev', brightness: 78, native_brightness: false, device_present: true, updated_unix: 1_789_344_000 }
      : {};
    await route.fulfill({ json });
  });
}

for (const width of [390, 768, 1440]) {
  test(`admin designer stays usable at ${width}px`, async ({ page }) => {
    await mockApi(page); await page.setViewportSize({ width, height: 1000 }); await page.goto('/');
    await expect(page.getByTestId('logical-canvas')).toBeVisible();
    await expect(page.getByTestId('page-strip')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Home Revision 3' })).toBeVisible();
    await expect(page.getByRole('button', { name: '+ Metric' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Templates' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Save carousel' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Preview', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Apply to LCD' })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await page.getByTestId('logical-canvas').evaluate((node) => { const rect = node.getBoundingClientRect(); return rect.left >= 0 && rect.right <= window.innerWidth + 1; })).toBe(true);
    if (width === 390) {
      const tabs = page.getByRole('navigation', { name: 'Admin sections' });
      for (const name of ['Pages', 'Media', 'Display', 'Providers']) {
        const tab = tabs.getByRole('button', { name });
        await expect(tab).toBeVisible();
        expect(await tab.evaluate((node) => { const rect = node.getBoundingClientRect(); return rect.left >= 0 && rect.right <= window.innerWidth; })).toBe(true);
        await tab.click();
        await expect(tab).toHaveAttribute('aria-pressed', 'true');
      }
      await tabs.getByRole('button', { name: 'Pages' }).click();
      expect((await page.getByTestId('page-strip').boundingBox())!.height).toBeLessThan(260);
      expect((await page.getByTestId('logical-canvas').boundingBox())!.y).toBeLessThan(650);
      const homeTile = page.getByTestId('page-tile-page-home');
      await homeTile.getByLabel('Page actions Home').click();
      await homeTile.getByRole('checkbox', { name: 'Enabled' }).uncheck();
      await homeTile.getByRole('spinbutton', { name: 'Duration Home' }).fill('15');
      await homeTile.getByRole('button', { name: 'Move Home down' }).click();
      await expect(page.locator('[data-testid^="page-tile-"]').first()).toHaveAttribute('data-testid', 'page-tile-page-storage');
      await homeTile.getByLabel('Page actions Home').click();
      expect(await page.evaluate(() => [...document.querySelectorAll<HTMLElement>('*')].filter((node) => node.scrollWidth > node.clientWidth + 1 && ['auto', 'scroll'].includes(getComputedStyle(node).overflowX)).length)).toBe(0);
    }
    await page.screenshot({ path: `${output}/pages-${width}.png`, fullPage: true });
  });
}

test('metric insertion stays friendly until Advanced is opened', async ({ page }) => {
  await mockApi(page); await page.setViewportSize({ width: 390, height: 1000 }); await page.goto('/');
  await page.getByRole('button', { name: '+ Metric' }).click();
  await page.getByRole('button', { name: 'Add CPU load' }).click();
  const canvas = page.getByTestId('logical-canvas');
  await expect(canvas.getByRole('button', { name: /CPU load 42%/ })).toBeVisible();
  await expect(page.getByText('aooscope_pve_cpu_pct')).toHaveCount(0);
  await page.getByRole('button', { name: 'Advanced' }).click();
  await expect(page.getByText('aooscope_pve_cpu_pct')).toBeVisible();
});

test('template picker posts template_id and selects the created page', async ({ page }) => {
  await mockApi(page); await page.setViewportSize({ width: 1440, height: 1000 }); await page.goto('/');
  await page.getByRole('button', { name: 'Templates' }).click();
  const requestPromise = page.waitForRequest((request) => request.url().endsWith('/api/pages') && request.method() === 'POST');
  await page.getByRole('button', { name: /Semi rings/ }).click();
  const request = await requestPromise;
  expect(request.postDataJSON()).toEqual({ template_id: 'factory.semi-rings.v1' });
  await expect(page.getByRole('heading', { name: 'Semi rings' })).toBeVisible();
});

test('functional admin tabs and 1440 review screenshots', async ({ page }) => {
  await mockApi(page); await page.setViewportSize({ width: 1440, height: 1000 }); await page.goto('/');
  const tabs = page.getByRole('navigation', { name: 'Admin sections' });
  await page.screenshot({ path: `${output}/pages.png`, fullPage: true });
  await tabs.getByRole('button', { name: 'Media' }).click(); await expect(page.getByText('Media Library')).toBeVisible(); await page.screenshot({ path: `${output}/media.png`, fullPage: true });
  await tabs.getByRole('button', { name: 'Display' }).click(); await expect(page.getByText(/Native backlight control is unsupported/)).toBeVisible(); await page.screenshot({ path: `${output}/display.png`, fullPage: true });
  await tabs.getByRole('button', { name: 'Providers' }).click(); await expect(page.getByText('Credential saved', { exact: true })).toBeVisible(); await expect(page.getByRole('button', { name: 'Test Proxmox' })).toBeVisible(); await page.screenshot({ path: `${output}/providers.png`, fullPage: true });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});
