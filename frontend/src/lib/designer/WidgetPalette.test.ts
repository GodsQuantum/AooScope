import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import WidgetPalette from './WidgetPalette.svelte';

describe('WidgetPalette', () => {
  it('keeps horizontal and vertical bars distinct', async () => {
    const onadd = vi.fn();
    render(WidgetPalette, { props: { onadd } });
    await fireEvent.click(screen.getByRole('button', { name: 'Ajouter Barre horizontale' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Ajouter Barre verticale' }));
    expect(onadd.mock.calls).toEqual([
      ['bar', { orientation: 'horizontal' }],
      ['bar', { orientation: 'vertical' }]
    ]);
  });
});
