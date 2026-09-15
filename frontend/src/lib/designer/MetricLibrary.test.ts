import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import MetricLibrary from './MetricLibrary.svelte';

const metric = {
  id: 'aooscope_hardware_cpu_temp_c', label: 'Température CPU', provider_name: 'Hardware local',
  category: 'CPU', value: 52, demo_value: 48, unit: '°C', recommended_widgets: ['gauge', 'value']
};

describe('MetricLibrary', () => {
  it('uses friendly metric content and inserts the recommended representation', async () => {
    const onadd = vi.fn();
    render(MetricLibrary, { props: { metrics: [metric], onadd } });
    expect(screen.getByText('Température CPU')).toBeTruthy();
    expect(screen.getByText('52 °C')).toBeTruthy();
    expect(screen.queryByText(metric.id)).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Add Température CPU' }));
    expect(onadd).toHaveBeenCalledWith(metric, 'gauge');
  });
});
