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
});
