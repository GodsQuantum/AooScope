import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import MediaCard from './MediaCard.svelte';

describe('MediaCard', () => {
  it('replaces a failed poster with a real placeholder', async () => {
    render(MediaCard, { props: { title: 'Movie', detail: 'detail', poster: '/poster.jpg' } });
    const image = screen.getByRole('img', { name: 'Poster for Movie' });
    await fireEvent.error(image);
    expect(screen.queryByRole('img')).toBeNull();
    expect(screen.getByLabelText('Poster unavailable')).toBeTruthy();
  });
});
