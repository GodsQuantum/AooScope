import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import PageStrip from './PageStrip.svelte';

const pages = [
  { id: 'home', name: 'Home', enabled: true, duration: 8, revision: 3, template_id: 'factory.home.v1' },
  { id: 'custom', name: 'Custom', enabled: false, duration: 12, revision: 1, template_id: null }
];

describe('PageStrip', () => {
  it('keeps carousel controls while presenting pages as a compact strip', async () => {
    const onchange = vi.fn();
    const onduplicate = vi.fn();
    const onrestore = vi.fn();
    const ondelete = vi.fn();
    render(PageStrip, { props: { pages, selected: 'home', onchange, onselect: vi.fn(), onduplicate, onrestore, ondelete, onsave: vi.fn(), oncreate: vi.fn() } });
    expect(screen.getByTestId('page-strip')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Home Revision 3' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Move Home down' }));
    expect(onchange).toHaveBeenCalledWith([pages[1], pages[0]]);
    await fireEvent.click(screen.getByRole('button', { name: 'Duplicate Home' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Restore Home' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Delete Home' }));
    expect(onduplicate).toHaveBeenCalledWith('home');
    expect(onrestore).toHaveBeenCalledWith('home');
    expect(ondelete).toHaveBeenCalledWith('home');
  });

  it('keeps tile settings inside the compact actions disclosure', async () => {
    const onchange = vi.fn();
    const onsave = vi.fn();
    const oncreate = vi.fn();
    render(PageStrip, { props: { pages, selected: 'home', onchange, onselect: vi.fn(), onduplicate: vi.fn(), onrestore: vi.fn(), ondelete: vi.fn(), onsave, oncreate } });
    expect(screen.getByTestId('page-thumbnail-home')).toBeTruthy();
    expect(screen.getByLabelText('Home enabled')).toBeTruthy();
    const details = screen.getAllByText('Actions')[0].closest('details');
    expect(details?.open).toBe(false);
    expect(details?.contains(screen.getAllByRole('checkbox', { name: 'Enabled' })[0])).toBe(true);
    expect(details?.contains(screen.getByRole('spinbutton', { name: 'Duration Home' }))).toBe(true);
    await fireEvent.click(screen.getAllByRole('checkbox', { name: 'Enabled' })[0]);
    expect(onchange).toHaveBeenCalledWith([{ ...pages[0], enabled: false }, pages[1]]);
    await fireEvent.change(screen.getByRole('spinbutton', { name: 'Duration Home' }), { target: { value: '15' } });
    expect(onchange).toHaveBeenCalledWith([{ ...pages[0], duration: 15 }, pages[1]]);
    await fireEvent.click(screen.getByRole('button', { name: 'Create page' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Save carousel' }));
    expect(oncreate).toHaveBeenCalled();
    expect(onsave).toHaveBeenCalled();
  });

  it('derives meaningful thumbnails from template ids and page names', () => {
    const examples = [
      { ...pages[0], id: 'rings', name: 'Metrics', template_id: 'factory.semi-rings.v1' },
      { ...pages[0], id: 'storage', name: 'Storage', template_id: 'factory.storage.v1' },
      { ...pages[1], id: 'custom', name: 'Night Ops', template_id: null }
    ];
    render(PageStrip, { props: { pages: examples, onchange: vi.fn(), onselect: vi.fn(), onduplicate: vi.fn(), onrestore: vi.fn(), ondelete: vi.fn(), onsave: vi.fn() } });
    expect(screen.getByTestId('page-thumbnail-rings').dataset.preview).toBe('rings');
    expect(screen.getByTestId('page-thumbnail-storage').dataset.preview).toBe('storage');
    expect(screen.getByTestId('page-thumbnail-custom').dataset.preview).toBe('custom');
    expect(screen.getByText('NO')).toBeTruthy();
  });
});
