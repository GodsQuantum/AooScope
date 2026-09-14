import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import CarouselSidebar from './CarouselSidebar.svelte';

const pages = [
  { id: 'home', name: 'Home', enabled: true, duration: 8, revision: 3, template_id: 'factory.home.v1' },
  { id: 'custom', name: 'Custom', enabled: false, duration: 12, revision: 1, template_id: null }
];

describe('CarouselSidebar', () => {
  it('exposes carousel metadata, editing, reorder and page actions', async () => {
    const onchange = vi.fn();
    const onduplicate = vi.fn();
    const onrestore = vi.fn();
    const ondelete = vi.fn();
    render(CarouselSidebar, { props: { pages, selected: 'home', onchange, onselect: vi.fn(), onduplicate, onrestore, ondelete, onsave: vi.fn() } });

    expect(screen.getByText('Revision 3')).toBeTruthy();
    expect((screen.getByRole('spinbutton', { name: 'Duration Home' }) as HTMLInputElement).value).toBe('8');
    await fireEvent.click(screen.getByRole('button', { name: 'Move Home down' }));
    expect(onchange).toHaveBeenCalledWith([pages[1], pages[0]]);
    await fireEvent.click(screen.getByRole('button', { name: 'Duplicate Home' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Restore Home' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Delete Home' }));
    expect(onduplicate).toHaveBeenCalledWith('home');
    expect(onrestore).toHaveBeenCalledWith('home');
    expect(ondelete).toHaveBeenCalledWith('home');
    expect(screen.getByRole('button', { name: 'Save carousel' })).toBeTruthy();
  });
});
