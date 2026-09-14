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
});
