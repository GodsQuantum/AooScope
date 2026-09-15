import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import TemplatePicker from './TemplatePicker.svelte';

describe('TemplatePicker', () => {
  it('offers the approved visual templates by human name', async () => {
    const oncreate = vi.fn();
    render(TemplatePicker, { props: { oncreate } });
    expect(screen.getByRole('button', { name: /Vertical bars/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Semi rings/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Horizontal bars/ })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: /Semi rings/ }));
    expect(oncreate).toHaveBeenCalledWith('factory.semi-rings.v1');
  });

  it('regenerates Storage from the live inventory action', async () => {
    const onstorage = vi.fn();
    render(TemplatePicker, { props: { oncreate: vi.fn(), onstorage } });
    await fireEvent.click(screen.getByRole('button', { name: /Storage cards/ }));
    expect(onstorage).toHaveBeenCalledOnce();
  });
});
