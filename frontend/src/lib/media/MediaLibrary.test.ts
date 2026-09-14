import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import MediaLibrary from './MediaLibrary.svelte';

describe('MediaLibrary', () => {
  const props = { onorbit: () => {}, onpreview: () => {} };

  it('uses the media file route for image thumbnails and falls back for other assets', () => {
    render(MediaLibrary, { props: { ...props, assets: [{ id: 'poster', name: 'Poster', kind: 'image' }, { id: 'orbit', name: 'Orbit', kind: 'animation' }] } });
    expect((screen.getByRole('img', { name: 'Poster' }) as HTMLImageElement).src).toContain('/api/media/poster/file');
    expect(screen.queryByRole('img', { name: 'Orbit' })).toBeNull();
    expect(screen.getByText('▶')).toBeTruthy();
  });
});
