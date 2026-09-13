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
});
