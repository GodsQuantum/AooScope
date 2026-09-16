import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import AdvancedInspector from './AdvancedInspector.svelte';

const metric = {
  id: 'aooscope_pve_cpu_pct', label: 'Utilisation CPU', provider_name: 'Proxmox', category: 'CPU',
  value: 42, demo_value: 50, unit: '%', recommended_widgets: ['gauge', 'value', 'bar']
};
const layer = { id: 'cpu', type: 'gauge' as const, binding: metric.id, x: 10, y: 20, width: 200, height: 120, z: 2 };

describe('AdvancedInspector', () => {
  it('keeps technical details out of the DOM until explicitly opened', async () => {
    render(AdvancedInspector, { props: { layer, metrics: [metric], onchange: vi.fn() } });
    expect(screen.queryByText(metric.id)).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Advanced settings' }));
    expect(screen.getByText(metric.id)).toBeTruthy();
    expect(screen.getByRole('spinbutton', { name: 'X position' })).toBeTruthy();
  });

  it('normalizes geometry edits to whole logical pixels', async () => {
    const onchange = vi.fn();
    render(AdvancedInspector, { props: { layer, metrics: [metric], onchange, initialOpen: true } });
    const input = screen.getByRole('spinbutton', { name: 'X position' });
    await fireEvent.input(input, { target: { value: '10.7' } });
    expect(onchange).toHaveBeenLastCalledWith({ x: 11 });
  });

  it('exposes image fit, anchor and aspect lock controls', async () => {
    const onchange = vi.fn();
    render(AdvancedInspector, { props: { layer: { id: 'logo', type: 'image', asset_id: 'a', fit: 'contain', align: 'center', valign: 'center', lock_aspect: true, aspect_ratio: 2, x: 0, y: 0, width: 200, height: 100 }, metrics: [], onchange, initialOpen: true } });
    expect(screen.getByLabelText('Image fit')).toBeTruthy();
    expect(screen.getByLabelText('Horizontal anchor')).toBeTruthy();
    expect(screen.getByLabelText('Vertical anchor')).toBeTruthy();
    expect(screen.getByLabelText('Lock aspect ratio')).toBeTruthy();
    await fireEvent.change(screen.getByLabelText('Image fit'), { target: { value: 'cover' } });
    expect(onchange).toHaveBeenLastCalledWith({ fit: 'cover' });
  });

  it('exposes shooting-star speed, trail, angle and compositing controls', () => {
    render(AdvancedInspector, { props: { layer: { id: 'star', type: 'shooting_star', x: 0, y: 0, width: 300, height: 100, z: 50 }, metrics: [], onchange: vi.fn(), initialOpen: true } });
    expect(screen.getByLabelText('Star speed')).toBeTruthy();
    expect(screen.getByLabelText('Trail length')).toBeTruthy();
    expect(screen.getByLabelText('Angle')).toBeTruthy();
    expect(screen.getByLabelText('Compositing')).toBeTruthy();
  });

});
