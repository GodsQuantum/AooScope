import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Tabs from './Tabs.svelte';

describe('Tabs', () => {
  it('exposes every admin section as a selectable tab', async () => {
    const onselect = vi.fn();
    render(Tabs, { props: { tabs: ['Pages', 'Media', 'Display', 'Providers'], active: 'Pages', onselect } });
    const tabs = screen.getAllByRole('button');
    expect(tabs.map((tab) => tab.textContent)).toEqual(['▦Pages', '▧Media', '◐Display', '⌁Providers']);
    expect(tabs[0].getAttribute('aria-pressed')).toBe('true');
    expect(tabs[3].getAttribute('aria-pressed')).toBe('false');
    await fireEvent.click(tabs[3]);
    expect(onselect).toHaveBeenCalledWith('Providers');
  });
});
