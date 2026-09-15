import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import DisplayControls from './DisplayControls.svelte';

describe('DisplayControls', () => {
  it('separates software luminance from hardware power', () => {
    render(DisplayControls, {
      props: {
        capabilities: { width: 960, height: 376, native_brightness: false, power_control: true },
        powerOn: true, brightness: 73
      }
    });
    expect(screen.getByText("Luminosité de l’image (logicielle)")).toBeTruthy();
    expect((screen.getByRole('slider') as HTMLInputElement).value).toBe('73');
    expect(screen.getByRole('button', { name: 'Éteindre l’écran' })).toBeTruthy();
    expect(screen.queryByLabelText('Luminosité native')).toBeNull();
  });

  it('keeps carousel interval visible while advanced display settings are collapsed', () => {
    render(DisplayControls, {
      props: {
        capabilities: { width: 960, height: 376, native_brightness: false, power_control: true },
        powerOn: true, brightness: 73, settings: { switch_seconds: 12 }
      }
    });
    expect(screen.getByLabelText('Carousel interval (seconds)')).toBeTruthy();
    const advanced = screen.getByText('Advanced display settings').closest('details');
    expect(advanced).toBeTruthy();
    expect(advanced?.hasAttribute('open')).toBe(false);
    expect(screen.getByText('Native backlight control is unsupported; this adjusts rendered image luminance only.')).toBeTruthy();
  });

  it('sends hardware power changes to the API', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ on: false }) });
    vi.stubGlobal('fetch', fetchMock);
    render(DisplayControls, {
      props: {
        capabilities: { width: 960, height: 376, native_brightness: false, power_control: true },
        powerOn: true, brightness: 73
      }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Éteindre l’écran' }));
    expect(fetchMock).toHaveBeenCalledWith('/api/display/power', expect.objectContaining({
      method: 'POST', body: JSON.stringify({ on: false })
    }));
    vi.unstubAllGlobals();
  });

  it('updates software luminance after a successful request', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ brightness: 41 }) });
    vi.stubGlobal('fetch', fetchMock);
    render(DisplayControls, { props: { capabilities: { width: 960, height: 376, native_brightness: false, power_control: true }, powerOn: true, brightness: 73 } });
    const input = screen.getByRole('slider');
    await fireEvent.input(input, { target: { value: '41' } });
    expect(fetchMock).toHaveBeenCalledWith('/api/display/luminance', expect.objectContaining({ method: 'POST', body: JSON.stringify({ value: 41 }) }));
    expect((input as HTMLInputElement).value).toBe('41');
    vi.unstubAllGlobals();
  });

  it('hydrates late settings without overwriting dirty local edits', async () => {
    const view = render(DisplayControls, { props: { capabilities: { width: 960, height: 376, native_brightness: false, power_control: true }, powerOn: true, brightness: 73, settings: {} } });
    const brand = screen.getByLabelText('Brand') as HTMLInputElement;
    await view.rerender({ settings: { brand: 'AooScope', timezone: 'Europe/Paris', switch_seconds: 12, schedule_enabled: true, schedule: [], brightness: 61 } });
    await waitFor(() => expect(brand.value).toBe('AooScope'));
    await fireEvent.input(brand, { target: { value: 'Local edit' } });
    await view.rerender({ settings: { brand: 'Server update', timezone: 'UTC', switch_seconds: 20, schedule_enabled: false, schedule: [], brightness: 50 } });
    expect(brand.value).toBe('Local edit');
  });

  it('treats schedule-only edits as dirty when late props arrive', async () => {
    const view = render(DisplayControls, { props: { capabilities: { width: 960, height: 376, native_brightness: false, power_control: true }, powerOn: true, brightness: 73, settings: { schedule: [{ start: '22:00', end: '08:00', brightness: 70 }] } } });
    const start = await screen.findByLabelText('Start') as HTMLInputElement;
    await fireEvent.input(start, { target: { value: '23:15' } });
    await view.rerender({ settings: { schedule: [{ start: '18:00', end: '06:00', brightness: 20 }] } });
    expect(start.value).toBe('23:15');
  });

  it('hydrates canonical and later clean parent settings after save', async () => {
    const canonical = { brand: 'Canonical', timezone: 'UTC', switch_seconds: 20, schedule_enabled: true, schedule: [], brightness: 61 };
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ display: canonical, providers: {} }) });
    vi.stubGlobal('fetch', fetchMock);
    const view = render(DisplayControls, { props: { capabilities: { width: 960, height: 376, native_brightness: false, power_control: true }, powerOn: true, brightness: 73, settings: { brand: 'Initial' }, settingsDocument: { display: {}, providers: {} } } });
    const brand = screen.getByLabelText('Brand') as HTMLInputElement;
    await fireEvent.input(brand, { target: { value: 'Local edit' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save display settings' }));
    await view.rerender({ settings: canonical, brightness: 61 });
    await waitFor(() => expect(brand.value).toBe('Canonical'));
    await view.rerender({ settings: { ...canonical, brand: 'Later update' }, brightness: 61 });
    await waitFor(() => expect(brand.value).toBe('Later update'));
  });

  it('keeps edits made while a successful save is pending', async () => {
    let resolveSave!: (value: { ok: boolean; json: () => Promise<unknown> }) => void;
    const save = new Promise<{ ok: boolean; json: () => Promise<unknown> }>((resolve) => { resolveSave = resolve; });
    const canonical = { brand: 'Submitted', timezone: 'UTC', switch_seconds: 20, schedule_enabled: true, schedule: [], brightness: 61 };
    vi.stubGlobal('fetch', vi.fn(() => save));
    const view = render(DisplayControls, { props: { capabilities: { width: 960, height: 376, native_brightness: false, power_control: true }, powerOn: true, brightness: 73, settings: { brand: 'Initial' }, settingsDocument: { display: {}, providers: {} } } });
    const brand = screen.getByLabelText('Brand') as HTMLInputElement;
    await fireEvent.input(brand, { target: { value: 'Submitted' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save display settings' }));
    await fireEvent.input(brand, { target: { value: 'Newer local edit' } });
    resolveSave({ ok: true, json: async () => ({ display: canonical, providers: {} }) });
    await screen.findByText('Display settings saved');
    await view.rerender({ settings: canonical, brightness: 61 });
    expect(brand.value).toBe('Newer local edit');
  });
});
