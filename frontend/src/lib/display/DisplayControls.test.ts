import { fireEvent, render, screen } from '@testing-library/svelte';
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
});
