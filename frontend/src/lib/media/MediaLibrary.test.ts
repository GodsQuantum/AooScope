import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import MediaLibrary from './MediaLibrary.svelte';

describe('MediaLibrary', () => {
  const props = { onorbit: () => {}, onpreview: () => {} };

  it('uses the media file route for image thumbnails and falls back for other assets', () => {
    render(MediaLibrary, { props: { ...props, assets: [{ id: 'poster', name: 'Poster', kind: 'image' }, { id: 'orbit', name: 'Orbit', kind: 'animation' }] } });
    expect((screen.getByRole('img', { name: 'Poster' }) as HTMLImageElement).src).toContain('/api/media/poster/file');
    expect(screen.queryByRole('img', { name: 'Orbit' })).toBeNull();
    expect(screen.getByText('▶')).toBeTruthy();
  });

  it('shows the actual splash source as a compact preset preview', () => {
    render(MediaLibrary, { props: { ...props, assets: [{ id: 'logo', name: 'Cloud logo', kind: 'image' }], presets: [{ id: 'orbit', name: 'Demo server · Orbit', source_asset_id: 'logo', settings: { fps: 8, speed_seconds: 4 } }] } });
    expect((screen.getByRole('img', { name: 'Preview Demo server · Orbit' }) as HTMLImageElement).src).toContain('/api/media/logo/file');
    expect(screen.queryByLabelText('Poster unavailable')).toBeNull();
  });

  it('uses generic splash wording while keeping the compatibility callback', async () => {
    const onorbit = vi.fn();
    render(MediaLibrary, { props: { onorbit, onpreview: () => {}, assets: [{ id: 'logo', name: 'Logo', kind: 'image' }] } });
    expect(screen.getByText('Splash animation')).toBeTruthy();
    expect(screen.queryByText(/Orbit animation preset|Créer \/ mettre à jour Orbit|Aperçu Orbit/)).toBeNull();
    await fireEvent.change(screen.getByLabelText('Splash source'), { target: { value: 'logo' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Use as splash' }));
    expect(onorbit).toHaveBeenCalledWith('logo', undefined);
  });
});
