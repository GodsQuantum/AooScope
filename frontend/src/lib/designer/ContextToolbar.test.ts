import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ContextToolbar from './ContextToolbar.svelte';

const metric = {
  id: 'aooscope_hardware_cpu_temp_c', label: 'Température CPU', provider_name: 'Hardware local',
  category: 'CPU', value: 52, demo_value: 48, unit: '°C', recommended_widgets: ['text', 'value', 'gauge', 'ring', 'bar', 'badge']
};
const layer = { id: 'cpu-temp', type: 'gauge' as const, binding: metric.id, x: 10, y: 10, width: 200, height: 120 };

describe('ContextToolbar', () => {
  it('edits visual representation without exposing the raw binding', async () => {
    const onchange = vi.fn();
    const onadvanced = vi.fn();
    render(ContextToolbar, { props: { layer, metrics: [metric], onchange, onadvanced } });
    expect(screen.getByText('Température CPU')).toBeTruthy();
    expect(screen.queryByText(metric.id)).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Bar' }));
    expect(onchange).toHaveBeenCalledWith({ type: 'bar' });
    await fireEvent.click(screen.getByRole('button', { name: 'Advanced' }));
    expect(onadvanced).toHaveBeenCalled();
  });

  it.each(['text', 'value', 'gauge', 'ring', 'bar', 'badge'] as const)('offers friendly sources for unbound and legacy %s layers', async (type) => {
    const onchange = vi.fn();
    const { unmount } = render(ContextToolbar, { props: { layer: { ...layer, type, binding: undefined }, metrics: [metric], onchange, onadvanced: vi.fn() } });
    expect([...screen.getByRole('combobox', { name: 'Metric source' }).querySelectorAll('option')].map((option) => option.text)).toContain('Température CPU · Hardware local');
    unmount();
    render(ContextToolbar, { props: { layer: { ...layer, type, binding: 'aooscope_missing_metric' }, metrics: [metric], onchange, onadvanced: vi.fn() } });
    const source = screen.getByRole('combobox', { name: 'Metric source' });
    expect([...source.querySelectorAll('option')].map((option) => option.text)).toContain('Température CPU · Hardware local');
    expect(document.body.textContent).not.toContain('aooscope_missing_metric');
    await fireEvent.change(source, { target: { value: metric.id } });
    expect(onchange).toHaveBeenCalledWith({ binding: metric.id });
  });

  it('does not offer an inert metric source for sparkline', () => {
    render(ContextToolbar, { props: { layer: { ...layer, type: 'sparkline' }, metrics: [metric], onchange: vi.fn(), onadvanced: vi.fn() } });
    expect(screen.queryByRole('combobox', { name: 'Metric source' })).toBeNull();
  });

  it('clears literal text when assigning a metric to a text layer', async () => {
    const onchange = vi.fn();
    render(ContextToolbar, { props: { layer: { ...layer, type: 'text', binding: undefined, text: 'Static heading' }, metrics: [metric], onchange, onadvanced: vi.fn() } });
    await fireEvent.change(screen.getByRole('combobox', { name: 'Metric source' }), { target: { value: metric.id } });
    expect(onchange).toHaveBeenCalledWith({ binding: metric.id, text: undefined });
    expect(Object.hasOwn(onchange.mock.calls[0][0], 'text')).toBe(true);
  });

  it.each([
    ['bar', 'Vertical', { orientation: 'vertical' }],
    ['gauge', 'Bold', { thickness: 28 }],
    ['ring', 'Slim', { thickness: 8 }],
    ['text', 'Center', { align: 'center' }],
    ['value', 'Right', { align: 'right' }],
    ['badge', 'Pill', { radius: 999 }]
  ] as const)('maps %s style choices to renderer properties', async (type, choice, expected) => {
    const onchange = vi.fn();
    render(ContextToolbar, { props: { layer: { ...layer, type }, metrics: [metric], onchange, onadvanced: vi.fn() } });
    await fireEvent.change(screen.getByRole('combobox', { name: 'Style' }), { target: { value: screen.getByRole('option', { name: choice }).getAttribute('value') } });
    expect(onchange).toHaveBeenLastCalledWith(expected);
    expect(onchange.mock.calls.flatMap(([changes]) => Object.keys(changes))).not.toContain('style');
  });
});
