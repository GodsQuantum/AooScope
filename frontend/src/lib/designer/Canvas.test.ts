import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Canvas from './Canvas.svelte';

describe('Canvas', () => {
  it('shows a resize handle for the selected layer', () => {
    vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
    render(Canvas, { props: {
      layers: [{ id: 'one', type: 'text', x: 10, y: 10, width: 40, height: 30 }],
      selected: 'one', onselect: vi.fn(), onchange: vi.fn()
    }});
    expect(screen.getByRole('button', { name: 'Redimensionner one' })).toBeTruthy();
  });

  it('keeps 960 by 376 logical coordinates while fitting a narrow host', async () => {
    let resize: (() => void) | undefined;
    vi.stubGlobal('ResizeObserver', class { constructor(callback: () => void) { resize = callback; } observe() {} disconnect() {} });
    render(Canvas, { props: { layers: [], onselect: vi.fn(), onchange: vi.fn() } });
    const host = screen.getByTestId('canvas-host');
    vi.spyOn(host, 'getBoundingClientRect').mockReturnValue({ width: 360, height: 141, left: 0, top: 0, right: 360, bottom: 141, x: 0, y: 0, toJSON: () => ({}) });
    resize?.();
    const canvas = screen.getByTestId('logical-canvas');
    expect(canvas.style.width).toBe('960px');
    expect(canvas.style.height).toBe('376px');
    await vi.waitFor(() => expect(canvas.style.transform).toBe('scale(0.375)'));
    expect(host.scrollWidth).toBeLessThanOrEqual(host.clientWidth);
  });
  it('shows friendly metric labels and values instead of raw bindings', () => {
    vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
    const metric = { id: 'aooscope_pve_cpu_pct', label: 'Utilisation CPU', provider_name: 'Proxmox', category: 'CPU', value: 42, demo_value: 50, unit: '%', recommended_widgets: ['gauge', 'value'] };
    render(Canvas, { props: {
      layers: [{ id: 'cpu', type: 'gauge', binding: metric.id, x: 10, y: 10, width: 180, height: 120 }],
      metrics: [metric], onselect: vi.fn(), onchange: vi.fn()
    }});
    expect(screen.getByText('Utilisation CPU')).toBeTruthy();
    expect(screen.getByText('42%')).toBeTruthy();
    expect(screen.queryByText(metric.id)).toBeNull();
  });

  it('previews renderer-backed orientation, thickness, alignment and radius', () => {
    vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
    render(Canvas, { props: {
      layers: [
        { id: 'bar', type: 'bar', orientation: 'vertical', x: 0, y: 0, width: 20, height: 100 },
        { id: 'ring', type: 'ring', thickness: 28, x: 30, y: 0, width: 100, height: 100 },
        { id: 'text', type: 'text', text: 'Aligned', align: 'right', x: 140, y: 0, width: 100, height: 30 },
        { id: 'badge', type: 'badge', text: 'Pill', radius: 999, x: 250, y: 0, width: 100, height: 30 }
      ],
      onselect: vi.fn(), onchange: vi.fn()
    }});
    expect(document.querySelector('[data-layer="bar"]')?.classList.contains('vertical')).toBe(true);
    expect((document.querySelector('[data-layer="ring"]') as HTMLElement).style.borderWidth).toBe('28px');
    expect((document.querySelector('[data-layer="text"]') as HTMLElement).style.textAlign).toBe('right');
    expect((document.querySelector('[data-layer="badge"]') as HTMLElement).style.borderRadius).toBe('999px');
  });

});
